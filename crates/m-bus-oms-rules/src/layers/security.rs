use super::{LayerFields, Link, RuleFinding, finding};
use alloc::vec::Vec;
use m_bus_core::ConfigurationField;

macro_rules! annex_f {
    ($page:literal, $top:literal) => {
        concat!(
            "https://oms-group.org/wp-content/uploads/2025/07/OMS-Spec_Vol2_AnnexF_C111.pdf#page=",
            $page,
            "&view=FitH,",
            $top
        )
    };
}

impl LayerFields {
    pub(super) fn afl_present(self) -> Option<bool> {
        if self.authentication_type.is_some() || self.outer_ci == Some(0x90) || self.reassembled {
            Some(true)
        } else if self.layers_decoded
            || self
                .outer_ci
                .is_some_and(|ci| !matches!(ci, 0x8c..=0x90 | 0xcf))
        {
            Some(false)
        } else {
            None
        }
    }

    pub(super) fn check_authentication(
        self,
        link: Link,
        configuration: Option<ConfigurationField>,
        has_afl: Option<bool>,
        results: &mut Vec<RuleFinding>,
    ) {
        if matches!(self.application_ci.or(self.outer_ci), Some(0x6e..=0x70))
            && configuration.is_none_or(|cf| cf.security_mode().to_bits() != 13)
        {
            results.push(finding("Application error protection", ("§8.8, Table 42", primary!(79, 600)),
                match (configuration, has_afl) {
                    (Some(cf), _) if cf.security_mode().to_bits() != 0 => Some(false),
                    (_, Some(true)) if self.authentication_type.is_some_and(|at| at != 0) => Some(false),
                    (_, Some(false)) if configuration.is_some() || self.application_ci.or(self.outer_ci) == Some(0x70) => Some(true),
                    _ => None,
                },
                "An unwrapped application error is sent in cleartext without authentication. Profile C has a separate exception."));
        }
        if link.function().is_some_and(crate::requires_mode_zero) {
            results.push(finding(
                "AFL on control messages",
                ("Table 43 note d", primary!(83, 145)),
                has_afl.map(|present| !present),
                "This link function requires a telegram without an AFL.",
            ));
        }
        if let Some(at) = self.authentication_type {
            results.push(finding(
                "AFL authentication type",
                ("§6.2.1", primary!(56, 321)),
                Some(at == 5),
                "Authenticated transmissions require AT 5 (eight-byte AES-CMAC).",
            ));
            if !self.reassembled {
                results.push(finding(
                    "Unfragmented AFL length field",
                    ("§6.2.5", primary!(56, 855)),
                    Some(self.message_length.is_none()),
                    "An unfragmented message must omit AFL.ML.",
                ));
            }
        }
        if let Some(length) = self.message_length {
            results.push(finding(
                "AFL message length",
                ("§6.2.5", primary!(56, 855)),
                Some(length <= 16384),
                "The message length may be at most 16 KiB.",
            ));
        }
        self.check_application_security(link, configuration, has_afl, results);
    }

    fn check_application_security(
        self,
        link: Link,
        configuration: Option<ConfigurationField>,
        has_afl: Option<bool>,
        findings: &mut Vec<RuleFinding>,
    ) {
        let Some(ci) = self.application_ci.or(self.outer_ci) else {
            return;
        };
        let mode = configuration.map(|cf| cf.security_mode().to_bits());
        if matches!(ci, 0x5f | 0x9e | 0x9f) {
            findings.push(finding(
                "Security management transport mode",
                ("§8.9", primary!(80, 165)),
                mode.map(|m| m == 13),
                "The OMS TLS security-management CI requires transport mode 13.",
            ));
        }
        let mandatory = matches!(ci, 0x53 | 0x55..=0x57 | 0x5a..=0x5b | 0x62 | 0x67..=0x68 | 0x6c..=0x6d | 0xc0..=0xc7);
        let clear = matches!(ci, 0x50..=0x52 | 0x54 | 0x66 | 0x71 | 0x80 | 0x8a..=0x8b | 0x93 | 0xb8 | 0xbb | 0xbd..=0xbf);
        let application_security_allowed = matches!(ci, 0x60..=0x61 | 0x64..=0x65 | 0x7c..=0x7f);
        let reference = (
            "§9.1, Table 42",
            if ci <= 0x6f {
                primary!(81, 423)
            } else {
                primary!(82, 110)
            },
        );
        let protected = mode.map(|mode| mode > 0);
        let authentication = match mode {
            Some(7) => self
                .authentication_type
                .map(|at| at == 5)
                .or(has_afl.filter(|v| !v)),
            Some(10) => self
                .configuration_extension
                .map(|ext| ext & 0x0300 == 0x0100),
            Some(13) => self.tls.map(|tls| tls.application),
            _ => self
                .authentication_type
                .map(|at| at != 0)
                .or(has_afl.filter(|v| !v)),
        };
        if mandatory || application_security_allowed {
            let enabled = protected == Some(true)
                || (self.lpwan_sync.is_none() && matches!(link, Link::Wireless(_)));
            findings.push(finding("Application protocol encryption", reference,
                protected.filter(|value| *value || (enabled && !application_security_allowed)),
                if application_security_allowed {
                    "This protocol permits application-specific encryption. Clear transport requires evidence from that application."
                } else {
                    "This application protocol requires a nonzero security mode when communication security is enabled."
                }));
            if mode != Some(5) {
                findings.push(finding("Application protocol authentication", reference,
                    authentication.filter(|value| *value || (enabled && !application_security_allowed)),
                    "An enabled security profile that supports authentication must authenticate this protocol."));
            }
        } else if clear {
            if self.tpl_present == Some(true) {
                findings.push(finding(
                    "Clear transport protocol",
                    reference,
                    mode.map(|m| m == 0),
                    "This transport protocol requires security mode zero.",
                ));
            }
            findings.push(finding(
                "Unauthenticated application protocol",
                reference,
                authentication.map(|a| !a),
                "This application protocol must omit authentication.",
            ));
        } else if matches!(ci, 0x72 | 0x7a | 0x74 | 0x75)
            && protected == Some(true)
            && mode != Some(5)
        {
            findings.push(finding("Application protocol authentication", reference, authentication,
                "This application response must use the authentication of its active security profile."));
        } else if matches!(ci, 0x74 | 0x75) && mode == Some(0) {
            findings.push(finding(
                "Unencrypted alarm authentication",
                reference,
                authentication.map(|a| !a),
                "An unencrypted alarm must omit authentication.",
            ));
        }
        if mode == Some(13) {
            self.check_tls(ci, has_afl, findings);
            if matches!(
                link.function(),
                Some(m_bus_core::Function::SndNr | m_bus_core::Function::AccNr)
            ) {
                findings.push(finding(
                    "Unsolicited TLS transmission",
                    ("Annex F.3.6.1", annex_f!(23, 90)),
                    Some(false),
                    "SND-NR and ACC-NR must be sent outside the TLS channel.",
                ));
            }
        }
    }

    fn check_tls(self, ci: u8, has_afl: Option<bool>, findings: &mut Vec<RuleFinding>) {
        let Some(tls) = self.tls else { return };
        let handshake_ci = matches!(ci, 0x5f | 0x9e | 0x9f);
        findings.push(finding("TLS record CI", ("Annex F.3.4.1, Table F.7", annex_f!(15, 115)),
            Some((!tls.channel_request || ci == 0x5f)
                && (!tls.handshake_or_alert || handshake_ci)
                && (!tls.application || !handshake_ci)),
            "Channel requests use CI 5F. Handshake and alert records use a handshake CI. Application data uses its application CI."));
        let cmac = self
            .authentication_type
            .map(|at| at == 5)
            .or(has_afl.filter(|v| !v));
        if tls.channel_request || tls.hello.is_some_and(|hello| hello.client) {
            findings.push(finding(
                "TLS handshake CMAC",
                ("Annex F.3.4.2–3, Tables F.10–12", annex_f!(17, 127)),
                cmac,
                "A channel request or ClientHello requires an eight-byte AFL CMAC.",
            ));
        }
        if self.authentication_type.is_some() {
            findings.push(finding(
                "TLS AFL authentication tag",
                ("Annex F.3.5.1, §9.3.3.1", annex_f!(19, 120)),
                tls.cmac_verified.then_some(true),
                "The communication key must verify the AFL CMAC over the TLS transport message.",
            ));
        }
        if let Some(hello) = tls.hello {
            let reference = ("Annex F.3.1", annex_f!(12, 110));
            findings.push(finding(
                "TLS hello sender",
                reference,
                Some(if hello.client {
                    matches!(ci, 0x9e | 0x9f)
                } else {
                    ci == 0x5f
                }),
                "The end device sends ClientHello and the gateway sends ServerHello.",
            ));
            findings.push(finding(
                "TLS truncated HMAC extension",
                reference,
                Some(!hello.truncated_hmac),
                "Client and server must omit the truncated-HMAC extension.",
            ));
            if let Some(uncompressed) = hello.uncompressed_points {
                findings.push(finding(
                    "TLS uncompressed point format",
                    reference,
                    Some(uncompressed),
                    "The declared EC point formats must allow uncompressed points.",
                ));
            }
            if let Some(required) = hello.required_curves {
                findings.push(finding("TLS required elliptic curves", ("Annex F.3.1, Table F.2", annex_f!(13, 90)), Some(required),
                    "An advertised supported-groups list must include secp256r1 and brainpoolP256r1."));
            }
            if let Some(size) = hello.maximum_fragment_length {
                findings.push(finding(
                    "TLS maximum fragment length",
                    reference,
                    Some((1..=4).contains(&size)),
                    "The maximum-fragment-length code must select 512, 1024, 2048 or 4096 bytes.",
                ));
            }
            if hello.client {
                findings.push(finding("TLS encrypt-then-MAC offer", reference, hello.encrypt_then_mac.then_some(true),
                    "Clients in meters newly installed from 2025 must offer encrypt-then-MAC. An omitted offer requires the installation date."));
            }
        }
        if tls.application {
            findings.push(finding(
                "TLS application CMAC",
                ("Annex F.3.4.5", annex_f!(18, 106)),
                has_afl.map(|present| !present),
                "TLS application data must omit AFL CMAC protection.",
            ));
        }
        if tls.alert {
            findings.push(finding("TLS alert protection", ("Annex F.3.4.4", annex_f!(17, 605)),
                if tls.clear_alert { cmac } else { None },
                "A clear TLS alert requires an AFL CMAC. An encrypted alert requires protection by an established session."));
        }
    }

    pub(super) fn check_security_fields(
        self,
        cf: ConfigurationField,
        findings: &mut Vec<RuleFinding>,
    ) {
        let mode = cf.security_mode().to_bits();
        let lower = cf.mode_specific_lower();
        let ext = self.configuration_extension;
        if (mode == 7 && lower >> 4 != 0) || (mode == 10 && lower != 0) {
            findings.push(finding("Transmitted security counter", ("§9.3.2.2, §9.3.2.4", primary!(89, 690)),
                self.security_counter.map(|counter| counter != 0), "A new authenticated and encrypted message increments the initial zero counter before transmission."));
        }
        if matches!(mode, 7 | 10) {
            findings.push(finding(
                "Communication security key ID",
                ("§9.2.2, Table 45", primary!(85, 102)),
                ext.map(|value| value & 15 <= 8),
                "The transport key ID must identify an assigned communication security key.",
            ));
        }
        if matches!(mode, 5 | 7) {
            findings.push(finding("CBC encrypted block count",
                ("§7.2.4.3–4", if mode == 5 { primary!(62, 257) } else { primary!(63, 573) }),
                self.transport_body_length.map(|length| length.checked_sub(usize::from(mode == 7)).is_some_and(|available| {
                    let blocks = lower >> 4;
                    if blocks == 15 { available % 16 == 0 } else { usize::from(blocks) * 16 <= available }
                })), "The declared CBC blocks must fit the payload. Full encryption requires a whole number of blocks."));
        }
        if mode == 10 {
            let room = self.transport_body_length.zip(ext).map(|(length, ext)| {
                let overhead = 2 + usize::from(ext & 0x40 != 0) + 4 + 8;
                length
                    .checked_sub(overhead)
                    .is_some_and(|available| lower == 0xff || usize::from(lower) <= available)
            });
            findings.push(finding("CCM encrypted byte count", ("§7.2.4.5, Tables 22–23", primary!(64, 670)), room,
                "The counter, optional key version, declared ciphertext and eight-byte tag must fit the transport body."));
        }
        if mode == 13 {
            findings.push(finding(
                "TLS transport length field",
                ("Annex F.3.3, Table F.4", annex_f!(13, 570)),
                Some(lower == 0xff),
                "The TLS transport requires FF in the low configuration byte.",
            ));
            findings.push(finding(
                "TLS protocol type",
                ("Annex F.3.3, Table F.5", annex_f!(14, 102)),
                ext.map(|ext| ext & 15 <= 1),
                "The extension must select a channel request or the TLS record protocol.",
            ));
        }
    }
}

impl super::ClockFields {
    pub fn assess(self, report: &mut crate::telegram::TelegramReport) {
        let set = matches!(self.control, 0 | 5 | 6);
        let reference = if set {
            ("Annex M, Table M.15", annex_m!(25, 89))
        } else {
            ("Annex M, Table M.14", annex_m!(24, 460))
        };
        report.push(finding(
            "SITP clock operation",
            reference,
            Some(matches!(self.control, 0..=3 | 5..=6)),
            "The clock control must select a listed adjust or set operation.",
        ));
        report.push(finding(
            "SITP clock command key",
            reference,
            Some(self.key == if set { 0x22 } else { 0x21 }),
            "The adjust and set commands require application key IDs 21 and 22 respectively.",
        ));
        report.push(finding(
            "SITP clock block control",
            reference,
            Some(matches!(self.block_control, 0x20 | 0xa0)),
            "The clock command block control must be 20 or A0.",
        ));
        report.push(finding(
            "SITP clock target time",
            reference,
            Some(self.immediate),
            "Clock commands require the immediate-action target time 3000000000.",
        ));
        report.push(finding(
            "SITP clock APDU length",
            reference,
            match self.control {
                0 => Some(self.length == 7),
                1..=3 => Some(self.length == 4),
                _ => None,
            },
            "The clock APDU must contain only its control and time value.",
        ));
        report.push(finding(
            "SITP clock application authentication",
            reference,
            self.verified.then_some(true),
            "The supplied application key must authenticate the ASP10 clock command.",
        ));
    }
}
