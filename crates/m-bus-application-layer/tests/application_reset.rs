use m_bus_application_layer::{parse_application_layer, ApplicationResetSubcode, UserDataBlock};

#[test]
fn reset_type_uses_the_upper_nibble_and_preserves_the_subtelegram_number() {
    use ApplicationResetSubcode::*;

    for (byte, expected) in [
        (0x07, All(0x07)),
        (0x10, UserData(0x10)),
        (0x17, UserData(0x17)),
        (0x27, SimpleBilling(0x27)),
        (0x37, EnhancedBilling(0x37)),
        (0x47, MultiTariffBilling(0x47)),
        (0x57, InstantaneousValues(0x57)),
        (0x67, LoadManagementValues(0x67)),
        (0x77, Reserved1(0x77)),
        (0x87, InstallationStartup(0x87)),
        (0x97, Testing(0x97)),
        (0xA7, Calibration(0xA7)),
        (0xF7, Reserved2(0xF7)),
    ] {
        let telegram = [0x50, byte];
        assert_eq!(
            parse_application_layer(&telegram).unwrap(),
            UserDataBlock::ResetAtApplicationLevel { subcode: expected }
        );
    }
}

#[cfg(feature = "std")]
#[test]
fn reset_type_has_a_display_label() {
    assert_eq!(ApplicationResetSubcode::from(0x17).to_string(), "User data");
}
