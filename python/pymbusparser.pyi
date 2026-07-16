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
    "annotated",
    "annotated-text",
    "hexview",
]

__version__: str
__all__: List[str]

def parse(data: DataInput, *, key: Optional[KeyInput] = None) -> Dict[str, Any]: ...
def parse_records(data: DataInput) -> List[Any]: ...
def render(
    data: DataInput,
    format: OutputFormat = "json",
    *,
    key: Optional[KeyInput] = None,
) -> str: ...
def parse_application_layer(data_record: str) -> str: ...
def m_bus_parse(data: str, format: OutputFormat, key: Optional[str] = None) -> str: ...
