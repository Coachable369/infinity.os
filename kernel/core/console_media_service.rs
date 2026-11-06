//! Scoped playback and read operations for the immutable bundled player process.
use crate::runtime::{
    capability::CapabilityType,
    execution::SecurityIdentity,
    native_memory::{copy_from_user, copy_to_user, PageTable},
};
use crate::{drivers::audio, storage};

pub const PLAYER: &[u8] = b"/System/media-player.py";
// ------------------------=
// FUNC: extension
// DESC: Accepts file types with decoders in the player build.
// ------------------=
fn extension(path: &[u8]) -> bool {
    let ext = crate::runtime::application_association::path_extension(path);
    [b"mp3".as_slice(), b"mp4", b"wav", b"m4a", b"mov", b"flac"]
        .iter()
        .any(|s| s.eq_ignore_ascii_case(ext))
}
// ------------------------=
// FUNC: bundled_sample
// DESC: Allows only the three immutable public demonstration media objects.
// ------------------=
fn bundled_sample(path: &[u8]) -> bool {
    matches!(path, b"/System/Infinity-Chime.wav" | b"/System/Infinity-Melody.mp3" | b"/System/Infinity-Motion.mp4")
}
// ------------------------=
// FUNC: readable
// DESC: Limits the library and each open to authorized personal objects and immutable snapshots.
// ------------------=
fn readable(path: &[u8], user: [u8; 16]) -> Option<storage::object::ObjectMetadata> {
    if !extension(path) {
        return None;
    }
    let (m, _) = storage::object_inspect_path(path).ok()?;
    (user != [0;16] && m.kind != storage::object::ObjectType::NamespaceNode
        && ((m.space == storage::object::Space::System && m.owner.0 == [0;16] && bundled_sample(path))
            || (m.space == storage::object::Space::Personal
                && crate::runtime::native_python_scope::readable(m.owner.0, user, true, path))))
    .then_some(m)
}
// ------------------------=
// FUNC: output_owner
// DESC: Separates player DMA ownership from speech and other session services.
// ------------------=
fn output_owner(mut session: [u8; 16]) -> SecurityIdentity {
    session[0] ^= 0x68;
    session[15] ^= 0xd3;
    SecurityIdentity(session)
}
// ------------------------=
// FUNC: stop
// DESC: Releases only this player's active audio stream.
// ------------------=
pub fn stop(session: [u8; 16]) {
    audio::stop_playback(output_owner(session));
}
// ------------------------=
// FUNC: dispatch
// DESC: Checks caller identity and authorization before reading media or copying any PCM from user memory.
// ------------------=
pub fn dispatch<T: PageTable>(
    allowed: bool,
    user: [u8; 16],
    session: [u8; 16],
    scratch: &mut [u8],
    pending: &mut Option<crate::runtime::object_navigation::ByteText<96>>,
    tables: &mut T,
    op: u64,
    a: [u64; 6],
) -> Option<i64> {
    if !(0x12e0..=0x12e7).contains(&op) {
        return None;
    }
    if !allowed {
        return Some(-13);
    }
    let owner = output_owner(session);
    let result = match op {
        0x12e0 => {
            let mut path = [0; 96];
            let mut seen = 0u64;
            for i in 0..storage::object::PUBLIC_NAMESPACE_CAPACITY {
                let Some((n, _)) = storage::namespace_entry(i, &mut path) else {
                    break;
                };
                if readable(&path[..n], user).is_none() {
                    continue;
                }
                if seen == a[0] {
                    return Some(
                        if a[2] >= n as u64 && copy_to_user(tables, a[1], &path[..n]).is_ok() {
                            n as i64
                        } else {
                            -14
                        },
                    );
                }
                seen += 1;
            }
            -2
        }
        0x12e1 => {
            if a[1] == 0 || a[1] > 95 {
                return Some(-22);
            }
            let mut path = [0; 95];
            let path = &mut path[..a[1] as usize];
            if copy_from_user(tables, a[0], path).is_err() {
                return Some(-14);
            }
            let Some(m) = readable(path, user) else {
                return Some(-13);
            };
            let n = m.logical_size as usize;
            if n > 4 * 1024 * 1024 || n > scratch.len() {
                return Some(-27);
            }
            if a[2] == 0 {
                return Some(n as i64);
            }
            if a[3] != n as u64 {
                return Some(-22);
            }
            let result =
                storage::object_read_path(path, Some(m.current_version), &mut scratch[..n]);
            let copied = matches!(result,Ok((id,len)) if id==m.object.id&&len==n)
                && copy_to_user(tables, a[2], &scratch[..n]).is_ok();
            scratch[..n].fill(0);
            if copied {
                n as i64
            } else {
                -5
            }
        }
        0x12e2 => audio::playback_rate().map_or(-19, i64::from),
        0x12e3 => {
            if a[1] == 0 || a[1] > 48000 * 4 * 2 || a[1] % 4 != 0 || a[2] > 100 {
                return Some(-22);
            }
            let now = crate::ui::performance::monotonic_ns().unwrap_or(0);
            if now == 0 {
                return Some(-5);
            }
            if !audio::media_output_available() {
                return Some(-11);
            }
            let cap = crate::runtime::with_runtime(|r| {
                r.capabilities.grant(
                    CapabilityType::AudioOutput,
                    0,
                    1,
                    0,
                    owner,
                    owner,
                    Some(now / 1_000_000_000 + 35),
                    0,
                )
            })
            .and_then(Result::ok);
            let Some(cap) = cap else {
                return Some(-13);
            };
            if !audio::infinity_audio_open(owner, cap, now) {
                crate::runtime::with_runtime(|r| {
                    let _ = r.capabilities.retire_leaf(cap, owner);
                });
                return Some(-11);
            }
            let mut pcm = [0i16; 2048];
            let mut offset = 0;
            while offset < a[1] {
                let count = (a[1] - offset).min(4096) as usize;
                let bytes = unsafe {
                    core::slice::from_raw_parts_mut(pcm.as_mut_ptr().cast::<u8>(), count)
                };
                let ok = a[0]
                    .checked_add(offset)
                    .is_some_and(|addr| copy_from_user(tables, addr, bytes).is_ok());
                if ok && a[2] != 100 {
                    for sample in &mut pcm[..count / 2] {
                        *sample = (i32::from(*sample) * a[2] as i32 / 100) as i16;
                    }
                }
                if !ok || !audio::infinity_audio_append(owner, cap, now, &pcm[..count / 2], 0, 0) {
                    stop(session);
                    return Some(-5);
                }
                offset += count as u64;
            }
            if audio::infinity_audio_seal(owner, now) {
                0
            } else {
                stop(session);
                -5
            }
        }
        0x12e4 => {
            stop(session);
            0
        }
        0x12e5 => {
            if let Some(frames) = audio::playback_frames(owner) {
                1 + frames as i64
            } else if audio::media_output_available() {
                0
            } else {
                -11
            }
        }
        0x12e7 => {
            let Some(name) = pending.as_ref() else {
                return Some(0);
            };
            let path = name.as_bytes();
            let len = path.len();
            if a[1] < len as u64 || copy_to_user(tables, a[0], path).is_err() {
                return Some(-14);
            }
            *pending = None;
            len as i64
        }
        _ => {
            use crate::ui::panel_recipe::{self, Material, Surface};
            let primary = a[1] & 1 != 0;
            let state = a[1] >> 1;
            if state > 2 {
                return Some(-22);
            }
            let colors = crate::ui::skin::kit_tokens::runtime_colors();
            let rgb = |v: u32| ((v >> 16) as u8, (v >> 8) as u8, v as u8);
            let material = Material {
                fill: rgb(colors[if primary { 6 } else { 1 }]),
                edge: rgb(colors[3]),
                opacity: 255,
                edge_opacity: 255,
                focus: if state == 0 { 0 } else { 255 },
                pressed: state == 2,
                scale: 1,
            };
            let kind = if primary {
                Surface::Primary
            } else {
                Surface::Secondary
            };
            let style = panel_recipe::surface_style(kind);
            let pixels = &mut scratch[..96 * 34 * 3];
            for y in 0..34 {
                for x in 0..96 {
                    let (r, g, b, alpha) =
                        panel_recipe::pixel_styled(kind, 96, 34, x, y, material, style);
                    for (i, (v, bg)) in [(r, 8u8), (g, 20), (b, 38)].into_iter().enumerate() {
                        pixels[(y * 96 + x) * 3 + i] = ((u32::from(v) * u32::from(alpha)
                            + u32::from(bg) * u32::from(255 - alpha))
                            / 255) as u8;
                    }
                }
            }
            if copy_to_user(tables, a[0], pixels).is_ok() {
                0
            } else {
                -14
            }
        }
    };
    Some(result)
}
