//! Read-only GPT inspection and scoped, confirmed personal-file cleanup.
use super::{ConsoleKey, ConsoleRuntime};
use crate::storage::{
    self,
    object::{ObjectType, Space},
};
use crate::ui::command_terminal::{self as terminal, disk_view::Row};

impl ConsoleRuntime {
    // ------------------------=
    // FUNC: disk_refresh
    // DESC: Reads a fresh partition or file snapshot only on navigation, never on repaint.
    // ------------------=
    fn disk_refresh(&self) {
        terminal::mutate(|s| {
            let view = &mut s.disk;
            view.count = 0;
            view.used = 0;
            view.pool_capacity = 0;
            if let Ok((used, total, _)) = storage::storage_usage() {
                view.used = u64::from(used) * 4096;
                view.pool_capacity = u64::from(total) * 4096;
            }
            if view.depth == 0 {
                match storage::visit_partitions(|name, first, bytes, mounted| {
                    view.push(name, bytes, first, [0; 16], mounted)
                }) {
                    Ok(capacity) => view.capacity = capacity,
                    Err(_) => {
                        view.count = 0;
                        view.capacity = 0;
                        view.message = b"Cannot read this disk's GPT.";
                    }
                }
            } else if view.partition.container {
                if let Ok((used, _, _)) = storage::storage_usage() {
                    view.used = u64::from(used) * 4096;
                }
                let mut path = [0; 96];
                for i in 0..storage::object::NAMESPACE_CAPACITY {
                    let Some((n, id)) = storage::namespace_entry(i, &mut path) else {
                        break;
                    };
                    let Ok((m, _)) = storage::object_inspect_path(&path[..n]) else {
                        continue;
                    };
                    if m.space == Space::Personal
                        && m.kind != ObjectType::NamespaceNode
                        && m.owner.0 == self.current_user.0
                    {
                        view.push(&path[..n], u64::from(m.logical_size), 0, id.0, false);
                    }
                }
            } else {
                let partition = view.partition;
                match storage::visit_partition_files(
                    partition.first,
                    partition.bytes,
                    |path, bytes| view.push(path, bytes, 0, [0; 16], false),
                ) {
                    Ok(()) => view.message = b"Read-only partition. Files sorted by size.",
                    Err(_) => view.message = b"Directory unavailable or unsupported filesystem.",
                }
            }
            if view.depth != 0 {
                view.rows[..view.count].sort_unstable_by(|a, b| {
                    b.bytes.cmp(&a.bytes).then_with(|| a.label().cmp(b.label()))
                });
            }
            view.selected = view.selected.min(view.count.saturating_sub(1));
            s.changed();
        });
    }
    // ------------------------=
    // FUNC: disk_open
    // DESC: Starts a private disk session in the local authenticated terminal.
    // ------------------=
    pub(super) fn disk_open(&self) {
        terminal::mutate(|s| {
            s.disk.active = true;
            s.disk.depth = 0;
            s.disk.selected = 0;
            s.disk.confirm = false;
            s.disk.inspect = false;
            s.disk.message = b"";
        });
        self.disk_refresh();
    }
    // ------------------------=
    // FUNC: disk_cleanup
    // DESC: Revalidates object identity and file ownership before either stage of deletion.
    // ------------------=
    fn disk_cleanup(&self, row: Row) -> &'static [u8] {
        if !self.workplace_authorized(true) {
            return b"An authorized session is required.";
        }
        let path = row.label();
        let Ok((m, _)) = storage::object_inspect_path(path) else {
            return b"File is no longer available.";
        };
        if m.object.id.0 != row.id
            || m.kind == ObjectType::NamespaceNode
            || m.space != Space::Personal
            || m.owner.0 != self.current_user.0
        {
            return b"Cannot change this file.";
        }
        if path.starts_with(b"/trash/") {
            if storage::trash_delete(path).is_ok() {
                return if storage::object_collect().is_ok() {
                    b"Deleted. Unreferenced blocks reclaimed."
                } else {
                    b"Deleted. Collection deferred."
                };
            }
        } else if storage::trash_move(path).is_ok() {
            return b"Moved to Trash. Delete the Trash entry to reclaim space.";
        }
        b"Storage could not complete this operation."
    }
    // ------------------------=
    // FUNC: disk_key
    // DESC: Navigates the disk hierarchy; deletion requires confirmation of the currently selected file.
    // ------------------=
    pub(super) fn disk_key(&self, key: ConsoleKey) -> bool {
        let (active, depth, row, confirm) = terminal::inspect(|s| {
            let v = &s.disk;
            (
                v.active,
                v.depth,
                v.rows
                    .get(v.selected)
                    .copied()
                    .filter(|_| v.selected < v.count),
                v.confirm,
            )
        });
        if !active {
            return false;
        }
        if confirm {
            let msg = if matches!(key, ConsoleKey::Character(b'y' | b'Y')) {
                row.map_or(b"No file selected.".as_slice(), |r| self.disk_cleanup(r))
            } else {
                b"Cancelled."
            };
            terminal::mutate(|s| {
                s.disk.confirm = false;
                s.disk.inspect = false;
                s.disk.message = msg;
            });
            self.disk_refresh();
            return true;
        }
        if matches!(key, ConsoleKey::Character(b'u' | b'U')) && depth == 1 {
            let restored = row.is_some_and(|r| {
                r.label().starts_with(b"/trash/")
                    && self.workplace_authorized(true)
                    && storage::object_inspect_path(r.label()).is_ok_and(|(m, _)| {
                        m.owner.0 == self.current_user.0
                            && m.object.id.0 == r.id
                            && m.space == Space::Personal
                    })
                    && storage::trash_restore(r.label()).is_ok()
            });
            terminal::mutate(|s| {
                s.disk.message = if restored {
                    b"Restored to original location."
                } else {
                    b"Select your own Trash entry to restore."
                }
            });
            self.disk_refresh();
            return true;
        }
        let mut refresh = false;
        terminal::mutate(|s| {
            let v = &mut s.disk;
            match key {
                ConsoleKey::Up if !v.inspect => v.selected = v.selected.saturating_sub(1),
                ConsoleKey::Down if !v.inspect => {
                    v.selected = (v.selected + 1).min(v.count.saturating_sub(1))
                }
                ConsoleKey::Home if !v.inspect => v.selected = 0,
                ConsoleKey::End if !v.inspect => v.selected = v.count.saturating_sub(1),
                ConsoleKey::Enter | ConsoleKey::Right if depth == 0 => {
                    if let Some(p) = row {
                        v.partition = p;
                        v.depth = 1;
                        v.selected = 0;
                        v.message = b"";
                        refresh = true;
                    }
                }
                ConsoleKey::Enter | ConsoleKey::Right if depth == 1 && row.is_some() => {
                    v.inspect = true
                }
                ConsoleKey::Escape | ConsoleKey::Left if v.inspect => v.inspect = false,
                ConsoleKey::Delete | ConsoleKey::Character(b'd' | b'D')
                    if depth == 1 && v.partition.container && row.is_some() =>
                {
                    v.confirm = true
                }
                ConsoleKey::Character(b'r' | b'R') => {
                    v.message = b"";
                    refresh = true;
                }
                ConsoleKey::Escape | ConsoleKey::Left if depth != 0 => {
                    v.depth = 0;
                    v.selected = 0;
                    v.inspect = false;
                    v.message = b"";
                    refresh = true;
                }
                ConsoleKey::Escape | ConsoleKey::Character(b'q' | b'Q') => v.active = false,
                _ => {}
            }
            s.changed();
        });
        if refresh {
            self.disk_refresh();
        }
        true
    }
}
