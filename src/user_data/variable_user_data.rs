
#[derive(Debug, Clone)]
struct DataRecordHeader {
    dib: DataInformationBlock,
    vib: ValueInformationBlock,
}

#[derive(Debug, Clone)]
struct DataInformationBlock {
    dif: u8,
    dife: Vec<u8>, 
}

#[derive(Debug, Clone)]
struct ValueInformationBlock {
    vif: u8,
    vife: Vec<u8>, 
}

#[derive(Debug, Clone)]
struct DataRecord {
    header: DataRecordHeader,
    data: Vec<u8>, 
}