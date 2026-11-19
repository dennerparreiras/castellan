//! Small pieces of user-facing copy that have to stay honest.

/// Format a byte count the way Windows Explorer does: binary units, labeled `GB`.
#[must_use]
pub fn format_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * 1024;
    const GIB: u64 = 1024 * 1024 * 1024;
    if bytes >= GIB {
        format_unit(bytes, GIB, "GB")
    } else if bytes >= MIB {
        format_unit(bytes, MIB, "MB")
    } else if bytes >= KIB {
        format_unit(bytes, KIB, "KB")
    } else {
        format!("{bytes} B")
    }
}

fn format_unit(bytes: u64, unit: u64, label: &str) -> String {
    let whole = bytes / unit;
    let frac = (bytes % unit) * 10 / unit;
    if frac == 0 {
        format!("{whole} {label}")
    } else {
        format!("{whole}.{frac} {label}")
    }
}

/// Language for persona lines. `--json` and non-interactive output do not use these.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    /// English, the default.
    En,
    /// Brazilian Portuguese.
    PtBr,
}

/// Situations that have a persona line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PersonaMoment {
    /// Nothing failed.
    WallsHold,
    /// The morning round finished and freed some bytes.
    MorningRound {
        /// Bytes actually freed.
        bytes: u64,
    },
    /// The system volume is nearly full.
    VolumeFilling {
        /// Used percent, 0..=100.
        percent_used: u8,
    },
    /// This is a debug build and real mutations are refused.
    DevBuild,
}

/// One persona line. Information first, then the voice.
#[must_use]
pub fn persona(language: Language, moment: PersonaMoment) -> String {
    match (language, moment) {
        (Language::En, PersonaMoment::WallsHold) => {
            "Nothing to fix here. The walls hold.".to_owned()
        }
        (Language::PtBr, PersonaMoment::WallsHold) => {
            "Nada a corrigir. As muralhas seguem de pé.".to_owned()
        }
        (Language::En, PersonaMoment::MorningRound { bytes }) => format!(
            "Morning round complete. {} of dust swept.",
            format_bytes(bytes)
        ),
        (Language::PtBr, PersonaMoment::MorningRound { bytes }) => format!(
            "Ronda da manhã concluída. {} de poeira varridos.",
            format_bytes(bytes)
        ),
        (Language::En, PersonaMoment::VolumeFilling { percent_used }) => {
            format!("One thing needs your attention: the system drive is {percent_used}% full.")
        }
        (Language::PtBr, PersonaMoment::VolumeFilling { percent_used }) => format!(
            "Uma coisa precisa da sua atenção: o disco do sistema está {percent_used}% cheio."
        ),
        (Language::En, PersonaMoment::DevBuild) => {
            "DEV BUILD — real-system actions disabled".to_owned()
        }
        (Language::PtBr, PersonaMoment::DevBuild) => {
            "BUILD DE DESENVOLVIMENTO — ações reais no sistema estão desligadas".to_owned()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_use_units() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(2_254_857_831), "2.1 GB");
    }

    #[test]
    fn persona_puts_the_fact_before_the_voice() {
        let line = persona(
            Language::En,
            PersonaMoment::VolumeFilling { percent_used: 94 },
        );
        assert!(line.contains("94%"));
        assert!(!line.contains('!'));
    }
}
