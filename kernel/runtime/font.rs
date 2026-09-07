#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FontClass {
    Sans,
    Serif,
    Monospace,
    Accessible,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FontDescriptor {
    pub id: u32,
    pub family: &'static [u8],
    pub resource: &'static [u8],
    pub class: FontClass,
    pub variable: bool,
}

const FONTS: [FontDescriptor; 46] = [
    font(
        1,
        b"Inter",
        b"System/Fonts/Inter-Variable.ttf",
        FontClass::Sans,
        true,
    ),
    font(
        2,
        b"Roboto",
        b"System/Fonts/Roboto-Variable.ttf",
        FontClass::Sans,
        true,
    ),
    font(
        3,
        b"Open Sans",
        b"System/Fonts/OpenSans-Variable.ttf",
        FontClass::Sans,
        true,
    ),
    font(
        4,
        b"Lato",
        b"System/Fonts/Lato-Regular.ttf",
        FontClass::Sans,
        false,
    ),
    font(
        5,
        b"Montserrat",
        b"System/Fonts/Montserrat-Variable.ttf",
        FontClass::Sans,
        true,
    ),
    font(
        6,
        b"Fira Sans",
        b"System/Fonts/FiraSans-Regular.ttf",
        FontClass::Sans,
        false,
    ),
    font(
        7,
        b"Fira Code",
        b"System/Fonts/FiraCode-Variable.ttf",
        FontClass::Monospace,
        true,
    ),
    font(
        8,
        b"JetBrains Mono",
        b"System/Fonts/JetBrainsMono-Variable.ttf",
        FontClass::Monospace,
        true,
    ),
    font(
        9,
        b"IBM Plex Sans",
        b"System/Fonts/IBMPlexSans-Variable.ttf",
        FontClass::Sans,
        true,
    ),
    font(
        10,
        b"IBM Plex Mono",
        b"System/Fonts/IBMPlexMono-Regular.ttf",
        FontClass::Monospace,
        false,
    ),
    font(
        11,
        b"Source Sans 3",
        b"System/Fonts/SourceSans3-Variable.ttf",
        FontClass::Sans,
        true,
    ),
    font(
        12,
        b"Source Serif 4",
        b"System/Fonts/SourceSerif4-Variable.ttf",
        FontClass::Serif,
        true,
    ),
    font(
        13,
        b"Noto Sans",
        b"System/Fonts/NotoSans-Variable.ttf",
        FontClass::Sans,
        true,
    ),
    font(
        14,
        b"Noto Serif",
        b"System/Fonts/NotoSerif-Variable.ttf",
        FontClass::Serif,
        true,
    ),
    font(
        15,
        b"Atkinson Hyperlegible",
        b"System/Fonts/AtkinsonHyperlegible-Regular.ttf",
        FontClass::Accessible,
        false,
    ),
    font(
        16,
        b"Alegreya Sans",
        b"System/Fonts/AlegreyaSans.ttf",
        FontClass::Sans,
        false,
    ),
    font(
        17,
        b"Cabin",
        b"System/Fonts/Cabin.ttf",
        FontClass::Sans,
        true,
    ),
    font(
        18,
        b"Cardo",
        b"System/Fonts/Cardo.ttf",
        FontClass::Serif,
        false,
    ),
    font(
        19,
        b"Comfortaa",
        b"System/Fonts/Comfortaa.ttf",
        FontClass::Sans,
        true,
    ),
    font(
        20,
        b"Crimson Pro",
        b"System/Fonts/CrimsonPro.ttf",
        FontClass::Serif,
        true,
    ),
    font(
        21,
        b"DM Sans",
        b"System/Fonts/DMSans.ttf",
        FontClass::Sans,
        true,
    ),
    font(
        22,
        b"EB Garamond",
        b"System/Fonts/EBGaramond.ttf",
        FontClass::Serif,
        true,
    ),
    font(
        23,
        b"Exo 2",
        b"System/Fonts/Exo2.ttf",
        FontClass::Sans,
        true,
    ),
    font(
        24,
        b"Inconsolata",
        b"System/Fonts/Inconsolata.ttf",
        FontClass::Monospace,
        true,
    ),
    font(
        25,
        b"Karla",
        b"System/Fonts/Karla.ttf",
        FontClass::Sans,
        true,
    ),
    font(
        26,
        b"Libre Baskerville",
        b"System/Fonts/LibreBaskerville.ttf",
        FontClass::Serif,
        true,
    ),
    font(
        27,
        b"Libre Franklin",
        b"System/Fonts/LibreFranklin.ttf",
        FontClass::Sans,
        true,
    ),
    font(
        28,
        b"Manrope",
        b"System/Fonts/Manrope.ttf",
        FontClass::Sans,
        true,
    ),
    font(
        29,
        b"Merriweather",
        b"System/Fonts/Merriweather.ttf",
        FontClass::Serif,
        true,
    ),
    font(
        30,
        b"Mulish",
        b"System/Fonts/Mulish.ttf",
        FontClass::Sans,
        true,
    ),
    font(
        31,
        b"Nunito",
        b"System/Fonts/Nunito.ttf",
        FontClass::Sans,
        true,
    ),
    font(
        32,
        b"Oswald",
        b"System/Fonts/Oswald.ttf",
        FontClass::Sans,
        true,
    ),
    font(
        33,
        b"Oxygen",
        b"System/Fonts/Oxygen.ttf",
        FontClass::Sans,
        false,
    ),
    font(
        34,
        b"Playfair Display",
        b"System/Fonts/PlayfairDisplay.ttf",
        FontClass::Serif,
        true,
    ),
    font(
        35,
        b"Poppins",
        b"System/Fonts/Poppins.ttf",
        FontClass::Sans,
        false,
    ),
    font(
        36,
        b"PT Sans",
        b"System/Fonts/PTSans.ttf",
        FontClass::Sans,
        false,
    ),
    font(
        37,
        b"Raleway",
        b"System/Fonts/Raleway.ttf",
        FontClass::Sans,
        true,
    ),
    font(
        38,
        b"Rubik",
        b"System/Fonts/Rubik.ttf",
        FontClass::Sans,
        true,
    ),
    font(
        39,
        b"Space Grotesk",
        b"System/Fonts/SpaceGrotesk.ttf",
        FontClass::Sans,
        true,
    ),
    font(
        40,
        b"Titillium Web",
        b"System/Fonts/TitilliumWeb.ttf",
        FontClass::Sans,
        true,
    ),
    font(
        41,
        b"Work Sans",
        b"System/Fonts/WorkSans.ttf",
        FontClass::Sans,
        true,
    ),
    font(
        42,
        b"Zilla Slab",
        b"System/Fonts/ZillaSlab.ttf",
        FontClass::Serif,
        false,
    ),
    font(
        43,
        b"Barlow",
        b"System/Fonts/Barlow.ttf",
        FontClass::Sans,
        false,
    ),
    font(
        44,
        b"Bebas Neue",
        b"System/Fonts/BebasNeue.ttf",
        FontClass::Sans,
        false,
    ),
    font(
        45,
        b"Lexend",
        b"System/Fonts/Lexend.ttf",
        FontClass::Accessible,
        true,
    ),
    font(
        46,
        b"Arimo",
        b"System/Fonts/Arimo-Regular.ttf",
        FontClass::Sans,
        false,
    ),
];

// ------------------------=
// FUNC: font
// DESC: Constructs one immutable typed System-space font descriptor.
// ------------------=
const fn font(
    id: u32,
    family: &'static [u8],
    resource: &'static [u8],
    class: FontClass,
    variable: bool,
) -> FontDescriptor {
    FontDescriptor {
        id,
        family,
        resource,
        class,
        variable,
    }
}

pub struct FontCatalog;

impl FontCatalog {
    // ------------------------=
    // FUNC: new
    // DESC: Creates the immutable native font catalog.
    // ------------------=
    pub const fn new() -> Self {
        Self
    }

    // ------------------------=
    // FUNC: count
    // DESC: Returns the number of default font families installed in System Space.
    // ------------------=
    pub const fn count(&self) -> usize {
        FONTS.len()
    }

    // ------------------------=
    // FUNC: nth
    // DESC: Returns one stable font descriptor for typed enumeration.
    // ------------------=
    pub fn nth(&self, index: usize) -> Option<FontDescriptor> {
        FONTS.get(index).copied()
    }

    // ------------------------=
    // FUNC: by_id
    // DESC: Resolves a stable font identifier without relying on its display name.
    // ------------------=
    pub fn by_id(&self, id: u32) -> Option<FontDescriptor> {
        FONTS.iter().find(|font| font.id == id).copied()
    }
}
