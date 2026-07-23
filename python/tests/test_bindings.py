import json
import unittest
from importlib.metadata import version

import pymbusparser


WIRED_FRAME = (
    "68 3D 3D 68 08 01 72 00 51 20 02 82 4D 02 04 00 88 00 00 "
    "04 07 00 00 00 00 0C 15 03 00 00 00 0B 2E 00 00 00 0B 3B "
    "00 00 00 0A 5A 88 12 0A 5E 16 05 0B 61 23 77 00 02 6C "
    "8C 11 02 27 37 0D 0F 60 00 67 16"
)

APPLICATION_RECORDS = "2F2F03740100000413FCE0F5054413FCE0F505426C11390F0100F02F2F2F2F2F"

ENCRYPTED_FRAME = (
    "2E44931578563412330333637A2A0020255923C95AAA26D1B2E7493BC2"
    "AD013EC4A6F6D3529B520EDFF0EA6DEFC955B29D6D69EBF3EC8A"
)
DECRYPTION_KEY = bytes.fromhex("0102030405060708090A0B0C0D0E0F11")


class BindingsTests(unittest.TestCase):
    def test_parse_returns_native_objects(self):
        parsed = pymbusparser.parse(WIRED_FRAME)
        frame_bytes = bytes.fromhex(WIRED_FRAME.replace(" ", ""))

        self.assertIsInstance(parsed, dict)
        self.assertIn("frame", parsed)
        self.assertEqual(parsed, pymbusparser.parse(frame_bytes))
        self.assertEqual(parsed, pymbusparser.parse(bytearray(frame_bytes)))

    def test_public_api_is_explicit(self):
        self.assertEqual(pymbusparser.__version__, version("pymbusparser"))
        self.assertEqual(
            pymbusparser.__all__,
            [
                "parse",
                "parse_records",
                "render",
                "parse_application_layer",
                "m_bus_parse",
                "__version__",
            ],
        )

    def test_parse_records_returns_a_list(self):
        records = pymbusparser.parse_records(APPLICATION_RECORDS)

        self.assertIsInstance(records, list)
        self.assertGreater(len(records), 0)

    def test_render_and_legacy_api_return_strings(self):
        rendered = pymbusparser.render(WIRED_FRAME, "json")
        legacy = pymbusparser.m_bus_parse(WIRED_FRAME, "json")

        self.assertIsInstance(rendered, str)
        self.assertEqual(json.loads(rendered), json.loads(legacy))

        xml = pymbusparser.render(WIRED_FRAME, "xml")
        self.assertIsInstance(xml, str)
        self.assertIn("<MBusData>", xml)
        self.assertIsInstance(
            pymbusparser.parse_application_layer(APPLICATION_RECORDS), str
        )

    def test_invalid_inputs_raise_python_exceptions(self):
        with self.assertRaisesRegex(ValueError, "valid wired or wireless"):
            pymbusparser.parse("0102")
        with self.assertRaisesRegex(ValueError, "valid wired or wireless"):
            pymbusparser.render("0102", "table")
        with self.assertRaisesRegex(ValueError, "exactly 16 bytes"):
            pymbusparser.parse(WIRED_FRAME, key="1234")
        with self.assertRaisesRegex(ValueError, "unsupported format"):
            pymbusparser.render(WIRED_FRAME, "not-a-format")
        with self.assertRaises(TypeError):
            pymbusparser.parse(object())

    def test_decryption_is_compiled_into_the_wheel(self):
        rendered = pymbusparser.render(
            ENCRYPTED_FRAME, "hexview", key=DECRYPTION_KEY
        )
        parsed = json.loads(rendered)

        self.assertIs(parsed["decrypted"], True)
        display_hex = "".join(f"{byte:02X}" for byte in parsed["bytes"])
        self.assertIn("0C1427048502046D32371F1502FD170000", display_hex)


if __name__ == "__main__":
    unittest.main()
