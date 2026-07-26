from typing import Any, Dict, List, Literal, Optional, Union

DataInput = Union[str, bytes, bytearray]
KeyInput = Union[str, bytes, bytearray]
OutputFormat = Literal[
    "json",
    "yaml",
    "yml",
    "csv",
    "table",
    "mermaid",
    "xml",
    "annotated",
    "annotated-text",
    "hexview",
]

class MbusParserError(ValueError): ...

__version__: str
__all__: List[str]

def parse(
    data: DataInput,
    *,
    key: Optional[KeyInput] = None,
    include_enrichment: bool = True,
) -> Dict[str, Any]: ...
def parse_records(data: DataInput) -> List[Any]: ...
def render(
    data: DataInput,
    format: OutputFormat = "json",
    *,
    key: Optional[KeyInput] = None,
    width: Optional[int] = None,
    include_enrichment: bool = True,
) -> str: ...
def parse_application_layer(data_record: str) -> str: ...
def m_bus_parse(data: str, format: OutputFormat, key: Optional[str] = None) -> str: ...
