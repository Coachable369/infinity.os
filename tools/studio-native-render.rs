//! Repository-local draft rendering transport; pixels come from unchanged native production painters.
use std::{path::{Path,PathBuf},sync::OnceLock};

struct PreviewAssets {
    values:Vec<(Vec<u8>,Vec<u8>)>,
    precision:Option<usize>,
    background:Vec<u8>,
    template:Vec<u8>,
}
static ASSETS:OnceLock<PreviewAssets>=OnceLock::new();

// ------------------------=
// FUNC: basename
// DESC: Matches canonical template artwork names without treating user-controlled asset identifiers as filesystem paths.
// ------------------=
fn basename(name:&[u8])->&[u8] {name.rsplit(|byte|*byte==b'/').next().unwrap_or(name)}

// ------------------------=
// FUNC: asset_override
// DESC: Supplies validated unsaved binary assets only in the host-preview build, never altering native OS execution.
// ------------------=
pub(crate) fn asset_override(name:&[u8])->Option<&'static [u8]> {
    let assets=ASSETS.get()?;
    assets.values.iter().find(|(key,_)|key==name)
        .or_else(||assets.values.iter().find(|(key,_)|basename(key)==basename(name)))
        .or_else(||assets.values.iter().find(|(key,_)|{
            let (a,b)=(basename(key),basename(name));
            a.len()>4 && b.len()>4 && (&a[a.len()-4..]==b".bmp" || &a[a.len()-4..]==b".png")
                && (&b[b.len()-4..]==b".bmp" || &b[b.len()-4..]==b".png")
                && a[..a.len()-4]==b[..b.len()-4]
        })).map(|(_,bytes)|bytes.as_slice())
}

// ------------------------=
// FUNC: precision_art
// DESC: Retrieves the validated pending atlas in constant time for per-pixel native image sampling.
// ------------------=
pub(crate) fn precision_art()->Option<&'static [u8]> {
    let assets=ASSETS.get()?;
    Some(assets.values[assets.precision?].1.as_slice())
}

// ------------------------=
// FUNC: background
// DESC: Supplies the validated draft wallpaper to the unchanged native panel entrance sampler.
// ------------------=
pub(crate) fn background()->&'static[u8] {ASSETS.get().map(|assets|assets.background.as_slice()).unwrap_or(&[])}

// ------------------------=
// FUNC: settings_template_bytes
// DESC: Gives the production Settings parser the validated unsaved document for this isolated preview process.
// ------------------=
pub(crate) fn settings_template_bytes()->Option<&'static[u8]> {Some(ASSETS.get()?.template.as_slice())}

// ------------------------=
// FUNC: read_bounded
// DESC: Reads only repository-local bounded inputs so malformed previews cannot allocate arbitrary files or leave the project tree.
// ------------------=
fn read_bounded(path:&Path,root:&Path,limit:u64)->Result<Vec<u8>,String> {
    let resolved=path.canonicalize().map_err(|error|format!("Cannot resolve {}: {error}",path.display()))?;
    if !resolved.starts_with(root) {return Err("Preview inputs must remain within the project".into());}
    if std::fs::metadata(&resolved).map_err(|error|error.to_string())?.len()>limit {
        return Err("Preview input exceeds its bounded native size".into());
    }
    std::fs::read(resolved).map_err(|error|error.to_string())
}

// ------------------------=
// FUNC: run
// DESC: Validates a bounded draft request, invokes native scene painting and atomically publishes an RGBA framebuffer.
// ------------------=
pub(crate) fn run()->Result<(),String> {
    let args:Vec<String>=std::env::args().skip(1).collect();
    let mut template=None;let mut output=None;let mut background=None;
    let mut kind=0u8;let mut screen=1u8;let mut width=1280usize;let mut height=800usize;
    let mut focus=2usize;let mut pressed=false;let mut phase=None;let mut assets=Vec::new();
    let mut state=crate::configuration_wizard_render::DraftServiceState::default();
    let root=std::env::current_dir().map_err(|error|error.to_string())?.canonicalize().map_err(|error|error.to_string())?;
    if !root.join("build-kit.toml").is_file() {return Err("Run native Studio preview from its InfinityOS project root".into());}
    let mut index=0;
    while index<args.len() {
        let option=&args[index];index+=1;
        if option=="--studio-native-render" {continue;}
        if option=="--pressed" {pressed=true;continue;}
        if option=="--simulate-services" {state.simulated=true;continue;}
        if option=="--confirmation" {state.confirmation=true;continue;}
        let value=args.get(index).ok_or_else(||format!("Missing value for {option}"))?;index+=1;
        match option.as_str() {
            "--template"=>template=Some(PathBuf::from(value)),
            "--output"=>output=Some(PathBuf::from(value)),
            "--background"=>background=Some(PathBuf::from(value)),
            "--kind"=>kind=match value.as_str(){"installer"=>0,"configuration"=>1,"settings"=>2,"boot"=>3,"login"=>4,_=>return Err("Unknown preview collection".into())},
            "--screen"=>screen=value.parse().map_err(|_|"Invalid preview screen")?,
            "--width"=>width=value.parse().map_err(|_|"Invalid preview width")?,
            "--height"=>height=value.parse().map_err(|_|"Invalid preview height")?,
            "--focus"=>focus=value.parse().map_err(|_|"Invalid preview focus")?,
            "--progress"=>state.percent=value.parse().map_err(|_|"Invalid preview progress")?,
            "--time-zone"=>state.zone=value.parse().map_err(|_|"Invalid preview time zone")?,
            "--date-focus"=>state.date_focus=value.parse().map_err(|_|"Invalid preview date focus")?,
            "--date-part"=>state.date_part=value.parse().map_err(|_|"Invalid preview date part")?,
            "--expanded-row"=>state.expanded=Some(value.parse().map_err(|_|"Invalid preview expanded row")?),
            "--scroll"=>state.scroll=value.parse().map_err(|_|"Invalid preview scroll")?,
            "--control-focus"=>state.control=value.parse().map_err(|_|"Invalid preview control")?,
            "--phase-ms"=>phase=Some(value.parse::<u64>().map_err(|_|"Invalid animation phase")?.min(60000)),
            "--atlas"|"--styles"|"--glyphs"|"--tokens"=>{
                let name=match option.as_str(){"--atlas"=>"eclipse-precision-v1.rgba","--styles"=>"uikit-styles.bin","--tokens"=>"uikit-tokens.bin",_=>"eclipse-access-glyphs-v1.rgba"};
                assets.push((name.to_owned(),PathBuf::from(value)));
            },
            "--asset"=>{
                let (name,path)=value.split_once('=').ok_or("Asset override requires name=path")?;
                assets.push((name.to_owned(),PathBuf::from(path)));
            },
            _=>return Err(format!("Unknown native preview option {option}")),
        }
    }
    if width==0 || height==0 || width>3840 || height>2160 || focus>2 {return Err("Preview viewport or focus exceeds native bounds".into());}
    if state.percent>100 || state.zone>11 || !(2..=4).contains(&state.date_focus) || state.date_part>4
        || state.expanded.is_some_and(|index|index>127) || state.scroll>10000 || state.control>127 {
        return Err("Simulated service state exceeds native bounds".into());
    }
    let bytes=read_bounded(&template.ok_or("Missing template path")?,&root,512*1024*1024)?;
    let mut values:Vec<(Vec<u8>,Vec<u8>)>=Vec::new();let mut precision=None;
    if assets.len()>256 {return Err("Too many pending preview assets".into());}
    for (name,path) in assets {
        if name.is_empty() || name.len()>512 {return Err("Invalid preview asset identifier".into());}
        let data=read_bounded(&path,&root,128*1024*1024)?;
        let base=basename(name.as_bytes());
        if base==b"eclipse-precision-v1.rgba" {
            if data.len()!=2*14*256*256*4 {return Err("Invalid Precision atlas dimensions".into());}
            precision=Some(values.len());
        }else if base==b"eclipse-access-glyphs-v1.rgba" {
            if data.len()!=16*64*64*4 {return Err("Invalid native glyph atlas dimensions".into());}
        }else if base==b"uikit-styles.bin" {
            if crate::ui::panel_recipe::decode_styles(&data).is_none() {return Err("Invalid native style payload".into());}
        }else if base==b"uikit-tokens.bin" {
            if crate::ui::skin::kit_tokens::decode(&data).is_none() {return Err("Invalid native token payload".into());}
        }else if crate::ui::bitmap::RuntimeBitmap::parse(&data).is_none() {
            return Err(format!("Native image override is not a supported BMP: {name}"));
        }
        if values.iter().any(|(key,_)|*key==name.as_bytes()) {return Err("Duplicate preview asset identifier".into());}
        values.push((name.into_bytes(),data));
    }
    let background=background.unwrap_or_else(||root.join(match kind {
        1=>"assets/ui-design-kit/default/eclipse-access-wallpaper-v1.bmp",
        2=>"assets/desktop/infinity-shell-wallpaper-v3.bmp",
        _=>"assets/boot/infinity-installer-background-v2.bmp",
    }));
    let background_data=read_bounded(&background,&root,128*1024*1024)?;
    if crate::ui::bitmap::RuntimeBitmap::parse(&background_data).is_none() {return Err("Invalid native preview background".into());}
    ASSETS.set(PreviewAssets{values,precision,background:background_data.clone(),template:bytes.clone()}).map_err(|_|"Preview process already initialized")?;
    let rgba=crate::configuration_wizard_render::render_draft(&bytes,kind,screen,width,height,&background_data,focus,pressed,phase,state)?;
    let output=output.ok_or("Missing output path")?;
    let parent=output.parent().ok_or("Output requires a parent directory")?.canonicalize().map_err(|error|error.to_string())?;
    if !(parent.starts_with(root.join("build")) || parent.starts_with(root.join("builds"))) || output.extension().and_then(|value|value.to_str())!=Some("rgba") {
        return Err("Native preview output must be a repository build RGBA artifact".into());
    }
    let name=output.file_name().ok_or("Missing output filename")?;
    let output=parent.join(name);
    let pending=output.with_extension("rgba.partial");
    std::fs::write(&pending,rgba).map_err(|error|error.to_string())?;
    std::fs::rename(pending,&output).map_err(|error|error.to_string())?;
    let geometry=crate::configuration_wizard_render::draft_geometry(&bytes,kind,screen,width,height,state)?;
    std::fs::write(output.with_extension("geometry.json"),geometry).map_err(|error|error.to_string())?;
    Ok(())
}
