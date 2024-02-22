use m_bus_parser::frames::{parse_frame, Address, FrameType, Function};


fn main() {
    
    let example = vec![ 
        0x68, 0x06, 0x06, 0x68, 
        0x53, 0xFE, 0x51, 
        0x01, 0x7A, 0x08, 
        0x25, 0x16,
    ];

    let frame = parse_frame(&example).unwrap();

    if let FrameType::ControlFrame { function, address, data } = frame {
        assert_eq!(address, Address::Broadcast { reply_required: true });
        assert_eq!(function, Function::SndUd { fcb: (false)});
        assert_eq!(data, &[0x51,0x01, 0x7A, 0x08]);
    }

}