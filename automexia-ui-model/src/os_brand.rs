//! Display-only OS branding shared by chrome and prompt tags.
//!
//! Font Logos codepoints from the bundled Nerd Font; no system font required.
//! This bounded inventory cannot select executables, credentials or WSL targets.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OsBrand {
    pub name: &'static str,
    pub glyph: &'static str,
    pub color: [u8; 3],
    aliases: &'static [&'static str],
}

macro_rules! brands {
    ($( $id:ident, $name:literal, $glyph:literal, $color:expr, [$($alias:literal),*]; )*) => {
        $(pub const $id: OsBrand = OsBrand {
            name: $name, glyph: $glyph, color: $color, aliases: &[$name, $($alias),*],
        };)*
        pub const ALL: &[OsBrand] = &[$($id),*];
    };
}

brands! {
    LINUX, "Linux", "\u{f31a}", [0xe8, 0xc5, 0x47], ["GNU/Linux"];
    WINDOWS, "Windows", "\u{e70f}", [0x62, 0xb0, 0xff], ["Microsoft Windows"];
    MACOS, "macOS", "\u{f302}", [0xbf, 0xc5, 0xce], ["Darwin", "Mac OS X"];
    MINT, "Linux Mint", "\u{f30e}", [0x87, 0xcf, 0x3e], ["linuxmint", "LMDE"];
    UBUNTU, "Ubuntu", "\u{f31b}", [0xe9, 0x54, 0x20], [];
    DEBIAN, "Debian", "\u{f306}", [0xd7, 0x0a, 0x53], ["Debian GNU/Linux"];
    FEDORA, "Fedora", "\u{f30a}", [0x51, 0xa2, 0xda], ["Fedora Linux"];
    ARCH, "Arch Linux", "\u{f303}", [0x17, 0x93, 0xd1], ["arch", "archlinux"];
    MANJARO, "Manjaro", "\u{f312}", [0x35, 0xbf, 0x5c], ["Manjaro Linux"];
    OPENSUSE, "openSUSE", "\u{f314}", [0x73, 0xba, 0x25], ["openSUSE Leap", "openSUSE Tumbleweed", "opensuse-leap", "opensuse-tumbleweed", "SUSE Linux Enterprise Server", "SUSE Linux Enterprise Desktop", "sles", "sled"];
    NIXOS, "NixOS", "\u{f313}", [0x7e, 0xba, 0xe4], [];
    ALPINE, "Alpine Linux", "\u{f300}", [0x0d, 0x59, 0x7f], ["alpine"];
    POP, "Pop!_OS", "\u{f32a}", [0x48, 0xb9, 0xc7], ["pop", "pop-os"];
    ELEMENTARY, "elementary OS", "\u{f309}", [0x64, 0xba, 0xff], ["elementary"];
    KALI, "Kali Linux", "\u{f327}", [0x55, 0x7c, 0x94], ["Kali GNU/Linux", "kali"];
    CENTOS, "CentOS", "\u{f304}", [0x93, 0x22, 0x79], ["CentOS Linux", "CentOS Stream"];
    REDHAT, "Red Hat Enterprise Linux", "\u{f316}", [0xee, 0x00, 0x00], ["rhel", "Red Hat Enterprise Linux Server", "Red Hat Enterprise Linux Workstation"];
    ROCKY, "Rocky Linux", "\u{f32b}", [0x10, 0xb9, 0x81], ["rocky"];
    ALMA, "AlmaLinux", "\u{f31d}", [0x0f, 0x42, 0x66], ["AlmaLinux OS"];
    GENTOO, "Gentoo", "\u{f30d}", [0x99, 0x99, 0xff], ["Gentoo Linux"];
    VOID, "Void Linux", "\u{f32e}", [0x47, 0x80, 0x61], ["void"];
    ENDEAVOUR, "EndeavourOS", "\u{f322}", [0x7f, 0x7f, 0xff], [];
    ZORIN, "Zorin OS", "\u{f32f}", [0x15, 0xa6, 0xf0], ["zorin"];
    MAGEIA, "Mageia", "\u{f310}", [0x23, 0x97, 0xd4], [];
    SLACKWARE, "Slackware", "\u{f318}", [0x70, 0x91, 0xdb], ["Slackware Linux"];
    SOLUS, "Solus", "\u{f32d}", [0x52, 0x99, 0xd6], [];
    DEEPIN, "Deepin", "\u{f321}", [0x00, 0x7c, 0xff], ["Deepin GNU/Linux"];
    DEVUAN, "Devuan", "\u{f307}", [0xb9, 0x87, 0xd5], ["Devuan GNU/Linux"];
    ARTIX, "Artix Linux", "\u{f31f}", [0x10, 0xa0, 0xcc], ["artix"];
    PARROT, "Parrot OS", "\u{f329}", [0x15, 0xbd, 0xcc], ["Parrot GNU/Linux", "parrot"];
    RASPBERRY, "Raspberry Pi OS", "\u{f315}", [0xc5, 0x1a, 0x4a], ["Raspbian GNU/Linux", "raspbian"];
    FREEBSD, "FreeBSD", "\u{f30c}", [0xe0, 0x50, 0x50], [];
    OPENBSD, "OpenBSD", "\u{f328}", [0xf2, 0xca, 0x30], [];
}

impl OsBrand {
    /// Match names/IDs and versioned WSL aliases without allocation. Never
    /// infer a distro from a title, an arbitrary substring or its parent OS.
    pub fn from_name(name: &str) -> Self {
        if !automexia_extension_api::valid_os_name(name) {
            return LINUX;
        }
        let name = name.trim();
        ALL.iter()
            .copied()
            .find(|brand| {
                brand.aliases.iter().any(|alias| {
                    name.eq_ignore_ascii_case(alias)
                        || name.get(..alias.len()).is_some_and(|prefix| {
                            prefix.eq_ignore_ascii_case(alias)
                                && name.get(alias.len()..).is_some_and(|tail| {
                                    tail.strip_prefix([' ', '-']).is_some_and(|version| {
                                        version
                                            .as_bytes()
                                            .first()
                                            .is_some_and(u8::is_ascii_digit)
                                    })
                                })
                        })
                })
            })
            .unwrap_or(LINUX)
    }

    pub fn host() -> Self {
        match std::env::consts::OS {
            "windows" => WINDOWS,
            "macos" => MACOS,
            "freebsd" => FREEBSD,
            "openbsd" => OPENBSD,
            _ => LINUX,
        }
    }

    pub fn accent(self, active: bool) -> [f32; 4] {
        [
            self.color[0] as f32 / 255.0,
            self.color[1] as f32 / 255.0,
            self.color[2] as f32 / 255.0,
            if active { 1.0 } else { 0.72 },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn os_branding_matches_names_ids_and_versions_without_parent_distro_guesses() {
        for (name, brand) in [
            ("Linux Mint", MINT),
            ("LMDE 6", MINT),
            ("linuxmint", MINT),
            ("Ubuntu-24.04", UBUNTU),
            ("Debian GNU/Linux", DEBIAN),
            ("Fedora Linux", FEDORA),
            ("Arch Linux", ARCH),
            ("macOS", MACOS),
            ("openSUSE Tumbleweed", OPENSUSE),
            ("Microsoft Windows", WINDOWS),
        ] {
            assert_eq!(OsBrand::from_name(name), brand, "{name}");
        }
        for name in [
            "user@ubuntu:/home",
            "/bin/bash",
            "Ubuntuish",
            "Custom Ubuntu fork",
            "Unknown Distribution",
            "Ubuntu\u{202e}",
            "Linux Mint\n",
            "",
        ] {
            assert_eq!(OsBrand::from_name(name), LINUX, "{name:?}");
        }
        assert_eq!(OsBrand::from_name(&"x".repeat(257)), LINUX);
        assert_eq!(MINT.color, [0x87, 0xcf, 0x3e]);
        assert_eq!(MINT.glyph, "\u{f30e}");
        for brand in ALL {
            assert_eq!(OsBrand::from_name(brand.name), *brand);
            assert_eq!(brand.glyph.chars().count(), 1);
        }
    }
}
