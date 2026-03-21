#[cfg(feature = "std")]
pub struct ManufacturerInfo {
    pub name: &'static str,
    pub website: &'static str,
    pub description: &'static str,
}

/// Look up a 3-letter M-Bus manufacturer code (FLAG/DLMS registered).
/// Returns `None` for unknown codes.
#[cfg(feature = "std")]
#[must_use]
pub fn lookup_manufacturer(code: &str) -> Option<ManufacturerInfo> {
    match code {
        "ABB" => Some(ManufacturerInfo {
            name: "ABB AB",
            website: "abb.com",
            description: "Power and automation technologies (Sweden)",
        }),
        "ACE" => Some(ManufacturerInfo {
            name: "Actaris Electricity",
            website: "itron.com",
            description: "Electricity metering (acquired by Itron)",
        }),
        "ACG" => Some(ManufacturerInfo {
            name: "Actaris Gas",
            website: "itron.com",
            description: "Gas metering (acquired by Itron)",
        }),
        "ACW" => Some(ManufacturerInfo {
            name: "Actaris Water & Heat",
            website: "itron.com",
            description: "Water and heat metering (acquired by Itron)",
        }),
        "AEG" => Some(ManufacturerInfo {
            name: "AEG",
            website: "aeg.com",
            description: "Electrical engineering (Germany)",
        }),
        "AEL" => Some(ManufacturerInfo {
            name: "Kohler",
            website: "",
            description: "Metering solutions (Turkey)",
        }),
        "AEM" => Some(ManufacturerInfo {
            name: "S.C. AEM S.A.",
            website: "",
            description: "Metering solutions (Romania)",
        }),
        "AMP" => Some(ManufacturerInfo {
            name: "Ampy Automation Digilog Ltd",
            website: "",
            description: "Electricity metering (UK)",
        }),
        "AMT" => Some(ManufacturerInfo {
            name: "Aquametro",
            website: "aquametro.com",
            description: "Water and heat metering (Switzerland)",
        }),
        "APS" => Some(ManufacturerInfo {
            name: "Apsis Kontrol Sistemleri",
            website: "apsis.com.tr",
            description: "Control systems (Turkey)",
        }),
        "BEC" => Some(ManufacturerInfo {
            name: "Berg Energiekontrollsysteme GmbH",
            website: "berg-energie.de",
            description: "Energy monitoring systems (Germany)",
        }),
        "BER" => Some(ManufacturerInfo {
            name: "Bernina Electronic AG",
            website: "",
            description: "Electronic metering",
        }),
        "BSE" => Some(ManufacturerInfo {
            name: "Basari Elektronik A.S.",
            website: "basari.com.tr",
            description: "Electronic metering (Turkey)",
        }),
        "BST" => Some(ManufacturerInfo {
            name: "BESTAS Elektronik Optik",
            website: "bestas.com.tr",
            description: "Electronic metering (Turkey)",
        }),
        "CBI" => Some(ManufacturerInfo {
            name: "Circuit Breaker Industries",
            website: "cbi.co.za",
            description: "Electrical metering (South Africa)",
        }),
        "CLO" => Some(ManufacturerInfo {
            name: "Clorius Raab Karcher Energie Service A/S",
            website: "",
            description: "Heat metering",
        }),
        "CON" => Some(ManufacturerInfo {
            name: "Conlog",
            website: "",
            description: "Prepayment metering solutions",
        }),
        "CZM" => Some(ManufacturerInfo {
            name: "Cazzaniga S.p.A.",
            website: "",
            description: "Water metering (Italy)",
        }),
        "DAN" => Some(ManufacturerInfo {
            name: "Danubia",
            website: "",
            description: "Metering solutions",
        }),
        "DFS" => Some(ManufacturerInfo {
            name: "Danfoss A/S",
            website: "danfoss.com",
            description: "Energy and climate solutions (Denmark)",
        }),
        "DME" => Some(ManufacturerInfo {
            name: "Diehl Metering",
            website: "diehl-metering.com",
            description: "Water and heat metering (Germany)",
        }),
        "DWZ" => Some(ManufacturerInfo {
            name: "Lorenz GmbH & Co. KG",
            website: "lorenz-meters.de",
            description: "Flow measurement (Germany)",
        }),
        "DZG" => Some(ManufacturerInfo {
            name: "Deutsche Zählergesellschaft",
            website: "dzg.de",
            description: "Electricity metering (Germany)",
        }),
        "EDM" => Some(ManufacturerInfo {
            name: "EDMI Pty. Ltd.",
            website: "edmi-meters.com",
            description: "Smart metering solutions (Australia)",
        }),
        "EFE" => Some(ManufacturerInfo {
            name: "Engelmann Sensor GmbH",
            website: "engelmann.de",
            description: "Heat and cooling energy meters (Germany)",
        }),
        "EKT" => Some(ManufacturerInfo {
            name: "PA KVANT J.S.",
            website: "",
            description: "Metering solutions (Russia)",
        }),
        "ELM" => Some(ManufacturerInfo {
            name: "Elektromed Elektronik Ltd",
            website: "elektromed.com.tr",
            description: "Electronic metering (Turkey)",
        }),
        "ELS" => Some(ManufacturerInfo {
            name: "Elster Group",
            website: "elster.com",
            description: "Gas, electricity and water metering",
        }),
        "ELV" => Some(ManufacturerInfo {
            name: "Elvaco AB",
            website: "elvaco.com",
            description: "Wireless M-Bus communication modules (Sweden)",
        }),
        "EMH" => Some(ManufacturerInfo {
            name: "EMH metering",
            website: "emh-metering.com",
            description: "Smart electricity meters (Germany)",
        }),
        "EMO" => Some(ManufacturerInfo {
            name: "Enermet",
            website: "enermet.de",
            description: "Energy metering (Germany)",
        }),
        "EMU" => Some(ManufacturerInfo {
            name: "EMU Elektronik AG",
            website: "emuag.ch",
            description: "Energy measurement (Switzerland)",
        }),
        "END" => Some(ManufacturerInfo {
            name: "ENDYS GmbH",
            website: "endys.de",
            description: "Metering systems (Germany)",
        }),
        "ENP" => Some(ManufacturerInfo {
            name: "Kiev Polytechnical Scientific Research",
            website: "",
            description: "Metering R&D (Ukraine)",
        }),
        "ENT" => Some(ManufacturerInfo {
            name: "ENTES Elektronik",
            website: "entes.com.tr",
            description: "Energy measurement (Turkey)",
        }),
        "ERL" => Some(ManufacturerInfo {
            name: "Erelsan Elektrik ve Elektronik",
            website: "",
            description: "Electrical metering (Turkey)",
        }),
        "ESM" => Some(ManufacturerInfo {
            name: "Starion Elektrik ve Elektronik",
            website: "",
            description: "Electrical metering (Turkey)",
        }),
        "ESY" => Some(ManufacturerInfo {
            name: "Easymeter",
            website: "easymeter.com",
            description: "Electricity metering (Germany)",
        }),
        "EUR" => Some(ManufacturerInfo {
            name: "Eurometers Ltd",
            website: "",
            description: "Utility metering",
        }),
        "EWT" => Some(ManufacturerInfo {
            name: "Elin Wasserwerkstechnik",
            website: "",
            description: "Water metering",
        }),
        "FED" => Some(ManufacturerInfo {
            name: "Federal Elektrik",
            website: "federal.com.tr",
            description: "Electrical equipment (Turkey)",
        }),
        "FML" => Some(ManufacturerInfo {
            name: "Siemens Measurements Ltd.",
            website: "siemens.com",
            description: "Metering (Siemens subsidiary)",
        }),
        "GAV" => Some(ManufacturerInfo {
            name: "Carlo Gavazzi",
            website: "gavazzi.com",
            description: "Energy monitoring and automation (Switzerland)",
        }),
        "GBJ" => Some(ManufacturerInfo {
            name: "Grundfos A/S",
            website: "grundfos.com",
            description: "Pump and flow solutions (Denmark)",
        }),
        "GEC" => Some(ManufacturerInfo {
            name: "GEC Meters Ltd.",
            website: "",
            description: "Electricity metering (UK)",
        }),
        "GMC" => Some(ManufacturerInfo {
            name: "GMC-I Gossen-Metrawatt",
            website: "gossenmetrawatt.com",
            description: "Measurement and energy technology (Germany)",
        }),
        "GSP" => Some(ManufacturerInfo {
            name: "Ingenieurbuero Gasperowicz",
            website: "m-bus.de",
            description: "M-Bus engineering (Germany)",
        }),
        "GTE" => Some(ManufacturerInfo {
            name: "Sensoco",
            website: "",
            description: "Metering solutions",
        }),
        "GWF" => Some(ManufacturerInfo {
            name: "GWF MessSysteme AG",
            website: "gwf.ch",
            description: "Gas and water measurement (Switzerland)",
        }),
        "HEG" => Some(ManufacturerInfo {
            name: "Hamburger Elektronik Gesellschaft",
            website: "",
            description: "Electronic metering (Germany)",
        }),
        "HEL" => Some(ManufacturerInfo {
            name: "Heliowatt",
            website: "",
            description: "Electricity metering",
        }),
        "HRZ" => Some(ManufacturerInfo {
            name: "HERZ Messtechnik GmbH",
            website: "herz-messtechnik.de",
            description: "Flow measurement (Germany)",
        }),
        "HTC" => Some(ManufacturerInfo {
            name: "Horstmann Timers and Controls Ltd.",
            website: "",
            description: "Timing and control (UK)",
        }),
        "HYD" => Some(ManufacturerInfo {
            name: "Hydrometer GmbH",
            website: "hydrometer.de",
            description: "Water metering (Germany)",
        }),
        "ICM" => Some(ManufacturerInfo {
            name: "Intracom",
            website: "intracom.gr",
            description: "Telecommunications and metering (Greece)",
        }),
        "IDE" => Some(ManufacturerInfo {
            name: "IMIT S.p.A.",
            website: "imit.it",
            description: "Control and measurement (Italy)",
        }),
        "INV" => Some(ManufacturerInfo {
            name: "Invensys Metering Systems AG",
            website: "",
            description: "Metering solutions",
        }),
        "ISK" => Some(ManufacturerInfo {
            name: "Iskraemeco",
            website: "iskraemeco.com",
            description: "Electricity and smart metering (Slovenia)",
        }),
        "IST" => Some(ManufacturerInfo {
            name: "ista SE",
            website: "ista.com",
            description: "Energy and water services (Germany)",
        }),
        "ITR" => Some(ManufacturerInfo {
            name: "Itron",
            website: "itron.com",
            description: "Smart metering and grid solutions (USA)",
        }),
        "ITW" => Some(ManufacturerInfo {
            name: "ista SE",
            website: "ista.com",
            description: "Energy services and submetering (Germany)",
        }),
        "IWK" => Some(ManufacturerInfo {
            name: "IWK Regler und Kompensatoren GmbH",
            website: "",
            description: "Control and compensation (Germany)",
        }),
        "JAN" => Some(ManufacturerInfo {
            name: "Janitza Electronics GmbH",
            website: "janitza.de",
            description: "Power quality and energy measurement (Germany)",
        }),
        "KAM" => Some(ManufacturerInfo {
            name: "Kamstrup A/S",
            website: "kamstrup.com",
            description: "Energy and water metering (Denmark)",
        }),
        "KHL" => Some(ManufacturerInfo {
            name: "Kohler",
            website: "",
            description: "Metering solutions (Turkey)",
        }),
        "KKE" => Some(ManufacturerInfo {
            name: "KK-Electronic A/S",
            website: "",
            description: "Electronic metering (Denmark)",
        }),
        "KNX" => Some(ManufacturerInfo {
            name: "KNX Association",
            website: "knx.org",
            description: "Building automation standard (Belgium)",
        }),
        "KRO" => Some(ManufacturerInfo {
            name: "Elster Kromschröder",
            website: "elster.com",
            description: "Gas metering and combustion (Germany)",
        }),
        "KST" => Some(ManufacturerInfo {
            name: "Kundo SystemTechnik GmbH",
            website: "kundo.de",
            description: "Heat cost allocators (Germany)",
        }),
        "LEM" => Some(ManufacturerInfo {
            name: "LEM HEME Ltd.",
            website: "lem.com",
            description: "Current and voltage transducers (UK)",
        }),
        "LGB" => Some(ManufacturerInfo {
            name: "Landis+Gyr UK",
            website: "landisgyr.com",
            description: "Energy management solutions (UK)",
        }),
        "LGD" => Some(ManufacturerInfo {
            name: "Landis+Gyr Germany",
            website: "landisgyr.com",
            description: "Energy management solutions (Germany)",
        }),
        "LGZ" => Some(ManufacturerInfo {
            name: "Landis+Gyr AG Zug",
            website: "landisgyr.com",
            description: "Energy management solutions (Switzerland)",
        }),
        "LHA" => Some(ManufacturerInfo {
            name: "Atlantic Meters",
            website: "atlantic-meters.com",
            description: "Metering solutions (South Africa)",
        }),
        "LML" => Some(ManufacturerInfo {
            name: "LUMEL S.A.",
            website: "lumel.com.pl",
            description: "Measurement and control (Poland)",
        }),
        "LSE" => Some(ManufacturerInfo {
            name: "Landis & Staefa Electronic",
            website: "siemens.com",
            description: "Building technologies and metering (Siemens)",
        }),
        "LSP" => Some(ManufacturerInfo {
            name: "Landis & Staefa Production",
            website: "siemens.com",
            description: "Building technologies and metering (Siemens)",
        }),
        "LSZ" => Some(ManufacturerInfo {
            name: "Siemens Building Technologies",
            website: "siemens.com",
            description: "Building automation and metering (Germany)",
        }),
        "LUG" => Some(ManufacturerInfo {
            name: "Landis+Gyr",
            website: "landisgyr.com",
            description: "Global energy management solutions",
        }),
        "MAD" => Some(ManufacturerInfo {
            name: "Maddalena S.r.l.",
            website: "maddalena.it",
            description: "Water metering (Italy)",
        }),
        "MEI" => Some(ManufacturerInfo {
            name: "H. Meinecke AG",
            website: "",
            description: "Water metering (now Invensys)",
        }),
        "MKS" => Some(ManufacturerInfo {
            name: "MAK-SAY Elektrik Elektronik",
            website: "",
            description: "Electrical metering (Turkey)",
        }),
        "MNS" => Some(ManufacturerInfo {
            name: "MANAS Elektronik",
            website: "manas.com.tr",
            description: "Electronic metering (Turkey)",
        }),
        "MPS" => Some(ManufacturerInfo {
            name: "Multiprocessor Systems Ltd",
            website: "mps.bg",
            description: "Metering systems (Bulgaria)",
        }),
        "MTC" => Some(ManufacturerInfo {
            name: "Metering Technology Corporation",
            website: "",
            description: "Metering solutions (USA)",
        }),
        "NIS" => Some(ManufacturerInfo {
            name: "Nisko Industries",
            website: "",
            description: "Metering solutions (Israel)",
        }),
        "NMS" => Some(ManufacturerInfo {
            name: "Nisko Advanced Metering Solutions",
            website: "",
            description: "Advanced metering (Israel)",
        }),
        "NRM" => Some(ManufacturerInfo {
            name: "Norm Elektronik",
            website: "",
            description: "Electronic metering (Turkey)",
        }),
        "NZR" => Some(ManufacturerInfo {
            name: "NZR GmbH",
            website: "nzr.de",
            description: "Energy metering systems (Germany)",
        }),
        "ONR" => Some(ManufacturerInfo {
            name: "ONUR Elektroteknik",
            website: "",
            description: "Electrical metering (Turkey)",
        }),
        "PAD" => Some(ManufacturerInfo {
            name: "PadMess GmbH",
            website: "padmess.de",
            description: "M-Bus data acquisition (Germany)",
        }),
        "PMG" => Some(ManufacturerInfo {
            name: "Spanner-Pollux GmbH",
            website: "",
            description: "Metering solutions (now Invensys)",
        }),
        "PRI" => Some(ManufacturerInfo {
            name: "Polymeters Response International Ltd.",
            website: "",
            description: "Utility metering (UK)",
        }),
        "RAM" => Some(ManufacturerInfo {
            name: "Rossweiner Armaturen und Messgeräte",
            website: "",
            description: "Metering instruments (Germany)",
        }),
        "RAS" => Some(ManufacturerInfo {
            name: "Hydrometer GmbH",
            website: "hydrometer.de",
            description: "Water metering (Germany)",
        }),
        "REL" => Some(ManufacturerInfo {
            name: "Relay GmbH",
            website: "relay.de",
            description: "Remote reading and M-Bus (Germany)",
        }),
        "RKE" => Some(ManufacturerInfo {
            name: "ista SE",
            website: "ista.com",
            description: "Energy services and submetering (Germany)",
        }),
        "SAP" => Some(ManufacturerInfo {
            name: "Sappel",
            website: "sappel.com",
            description: "Water metering (France)",
        }),
        "SBC" => Some(ManufacturerInfo {
            name: "Saia-Burgess Controls",
            website: "saia-pcd.com",
            description: "Building automation controllers (Switzerland)",
        }),
        "SCH" => Some(ManufacturerInfo {
            name: "Schnitzel GmbH",
            website: "",
            description: "Metering solutions (Germany)",
        }),
        "SEN" => Some(ManufacturerInfo {
            name: "Sensus",
            website: "sensus.com",
            description: "Water and gas metering",
        }),
        "SEO" => Some(ManufacturerInfo {
            name: "Sensoco",
            website: "",
            description: "Metering solutions",
        }),
        "SIE" => Some(ManufacturerInfo {
            name: "Siemens AG",
            website: "siemens.com",
            description: "Electrification, automation and digitalization (Germany)",
        }),
        "SLB" => Some(ManufacturerInfo {
            name: "Schlumberger Industries",
            website: "slb.com",
            description: "Energy and water metering",
        }),
        "SME" => Some(ManufacturerInfo {
            name: "Siame",
            website: "",
            description: "Metering solutions (Tunisia)",
        }),
        "SML" => Some(ManufacturerInfo {
            name: "Siemens Measurements Ltd.",
            website: "siemens.com",
            description: "Metering (Siemens subsidiary, UK)",
        }),
        "SOF" => Some(ManufacturerInfo {
            name: "softflow.de GmbH",
            website: "softflow.de",
            description: "M-Bus software and metering (Germany)",
        }),
        "SON" => Some(ManufacturerInfo {
            name: "Sontex SA",
            website: "sontex.ch",
            description: "Heat and water metering (Switzerland)",
        }),
        "SPL" => Some(ManufacturerInfo {
            name: "Sappel",
            website: "sappel.com",
            description: "Water metering (France)",
        }),
        "SPX" => Some(ManufacturerInfo {
            name: "Spanner & Pluss",
            website: "spanner-pluss.de",
            description: "Gas metering (Germany)",
        }),
        "SVM" => Some(ManufacturerInfo {
            name: "AB Svensk Värmemätning SVM",
            website: "",
            description: "Heat metering (Sweden)",
        }),
        "TCH" => Some(ManufacturerInfo {
            name: "Techem",
            website: "techem.com",
            description: "Heat cost allocators and submetering (Germany)",
        }),
        "TIP" => Some(ManufacturerInfo {
            name: "TIP Thüringer Industrie Produkte GmbH",
            website: "stromzaehler.de",
            description: "Electricity metering (Germany)",
        }),
        "UAG" => Some(ManufacturerInfo {
            name: "Uher",
            website: "",
            description: "Metering solutions",
        }),
        "UGI" => Some(ManufacturerInfo {
            name: "United Gas Industries",
            website: "",
            description: "Gas metering",
        }),
        "VES" => Some(ManufacturerInfo {
            name: "ista SE",
            website: "ista.com",
            description: "Energy services and submetering (Germany)",
        }),
        "VPI" => Some(ManufacturerInfo {
            name: "Van Putten Instruments B.V.",
            website: "",
            description: "Measurement instruments (Netherlands)",
        }),
        "WEP" => Some(ManufacturerInfo {
            name: "Wepsta",
            website: "",
            description: "Heat metering",
        }),
        "WMO" => Some(ManufacturerInfo {
            name: "Westermo Teleindustri AB",
            website: "westermo.se",
            description: "Industrial data communications (Sweden)",
        }),
        "WZG" => Some(ManufacturerInfo {
            name: "Modularis",
            website: "",
            description: "Metering solutions",
        }),
        "YTE" => Some(ManufacturerInfo {
            name: "Yuksek Teknoloji",
            website: "",
            description: "High technology metering (Turkey)",
        }),
        "ZAG" => Some(ManufacturerInfo {
            name: "Zellwerg Uster AG",
            website: "",
            description: "Metering solutions (Switzerland)",
        }),
        "ZAP" => Some(ManufacturerInfo {
            name: "Zaptronix",
            website: "",
            description: "Metering solutions",
        }),
        "ZIV" => Some(ManufacturerInfo {
            name: "ZIV Aplicaciones y Tecnologia S.A.",
            website: "ziv.es",
            description: "Smart grid and metering (Spain)",
        }),
        "ZRI" => Some(ManufacturerInfo {
            name: "ZENNER International GmbH & Co. KG",
            website: "zenner.com",
            description: "Water, heat and gas metering (Germany)",
        }),
        "ZRM" => Some(ManufacturerInfo {
            name: "Minol Messtechnik",
            website: "minol.de",
            description: "Heat cost allocators and submetering (Germany)",
        }),
        _ => None,
    }
}
