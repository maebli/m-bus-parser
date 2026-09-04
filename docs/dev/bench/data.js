window.BENCHMARK_DATA = {
  "lastUpdate": 1788505405177,
  "repoUrl": "https://github.com/maebli/m-bus-parser",
  "entries": {
    "Parser stack and footprint": [
      {
        "commit": {
          "author": {
            "name": "Maebli",
            "username": "maebli"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7fb01ef5909be1dc3e1856989315902bc7413ba1",
          "message": "Merge pull request #116 from maebli/feature/zero-copy-completion\n\nFeature/zero copy completion (still a bit to go but one step closer)",
          "timestamp": "2026-09-01T22:59:52+02:00",
          "tree_id": "c84ab0e35dd146128d98af67acdeb56579a255fe",
          "url": "https://github.com/maebli/m-bus-parser/commit/7fb01ef5909be1dc3e1856989315902bc7413ba1"
        },
        "date": 1788296467072,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Eager full wired-frame parse stack",
            "value": 2784,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16; fixture=520 B; max(setup=476 B,records=2264 B)"
          },
          {
            "name": "Frame and application setup nested stack",
            "value": 476,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16; mbus=248 B; wired=64 B; application=228 B"
          },
          {
            "name": "Record decode nested stack",
            "value": 2264,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "Eager full-frame fixture local stack frame",
            "value": 520,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecords::next local stack frame",
            "value": 704,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecord::try_from local stack frame",
            "value": 16,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecord::parse local stack frame",
            "value": 456,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecordHeader::try_from local stack frame",
            "value": 280,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "ProcessedDataRecordHeader::try_from local stack frame",
            "value": 584,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "ValueInformation::try_from local stack frame",
            "value": 160,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "consume_orthhogonal_vife local stack frame",
            "value": 64,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "VIF block parser local stack frame",
            "value": 48,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecord value size",
            "value": 216,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "VIF block value size",
            "value": 20,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "Linked eager full parser text + data size",
            "value": 18887,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16; profile=opt-level=z,lto=fat,codegen-units=1; sections=text+data"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Michael Aebli",
            "username": "maebli"
          },
          "committer": {
            "name": "Michael Aebli",
            "username": "maebli"
          },
          "distinct": true,
          "id": "893b8f4c35bf52e482654aab31010274ad67dabb",
          "message": "Fixing pipeline for benches",
          "timestamp": "2026-09-03T21:54:36+02:00",
          "tree_id": "3ef63fddae036ac7a9353d9741fde7d14e8aecfd",
          "url": "https://github.com/maebli/m-bus-parser/commit/893b8f4c35bf52e482654aab31010274ad67dabb"
        },
        "date": 1788465369787,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Eager full wired-frame parse stack",
            "value": 2784,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16; fixture=520 B; max(setup=476 B,records=2264 B)"
          },
          {
            "name": "Frame and application setup nested stack",
            "value": 476,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16; mbus=248 B; wired=64 B; application=228 B"
          },
          {
            "name": "Record decode nested stack",
            "value": 2264,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "Eager full-frame fixture local stack frame",
            "value": 520,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecords::next local stack frame",
            "value": 704,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecord::try_from local stack frame",
            "value": 16,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecord::parse local stack frame",
            "value": 456,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecordHeader::try_from local stack frame",
            "value": 280,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "ProcessedDataRecordHeader::try_from local stack frame",
            "value": 584,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "ValueInformation::try_from local stack frame",
            "value": 160,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "consume_orthhogonal_vife local stack frame",
            "value": 64,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "VIF block parser local stack frame",
            "value": 48,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecord value size",
            "value": 216,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "VIF block value size",
            "value": 20,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "Linked eager full parser text + data size",
            "value": 18887,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16; profile=opt-level=z,lto=fat,codegen-units=1; sections=text+data"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Michael Aebli",
            "username": "maebli"
          },
          "committer": {
            "name": "Michael Aebli",
            "username": "maebli"
          },
          "distinct": true,
          "id": "1364f2041d6373cfa8b39c3566e58f10efc8bf8a",
          "message": "adjusting examples",
          "timestamp": "2026-09-03T22:14:51+02:00",
          "tree_id": "88eb8753c99beb0e3ff0697907a1e77317eeeb2e",
          "url": "https://github.com/maebli/m-bus-parser/commit/1364f2041d6373cfa8b39c3566e58f10efc8bf8a"
        },
        "date": 1788466566761,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Eager full wired-frame parse stack",
            "value": 2784,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16; fixture=520 B; max(setup=476 B,records=2264 B)"
          },
          {
            "name": "Frame and application setup nested stack",
            "value": 476,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16; mbus=248 B; wired=64 B; application=228 B"
          },
          {
            "name": "Record decode nested stack",
            "value": 2264,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "Eager full-frame fixture local stack frame",
            "value": 520,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecords::next local stack frame",
            "value": 704,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecord::try_from local stack frame",
            "value": 16,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecord::parse local stack frame",
            "value": 456,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecordHeader::try_from local stack frame",
            "value": 280,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "ProcessedDataRecordHeader::try_from local stack frame",
            "value": 584,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "ValueInformation::try_from local stack frame",
            "value": 160,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "consume_orthhogonal_vife local stack frame",
            "value": 64,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "VIF block parser local stack frame",
            "value": 48,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecord value size",
            "value": 216,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "VIF block value size",
            "value": 20,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "Linked eager full parser text + data size",
            "value": 18887,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16; profile=opt-level=z,lto=fat,codegen-units=1; sections=text+data"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Michael Aebli",
            "username": "maebli"
          },
          "committer": {
            "name": "Michael Aebli",
            "username": "maebli"
          },
          "distinct": true,
          "id": "b435ea42e3fe33211d021db5ea3d2f80fca9f168",
          "message": "Merge remote-tracking branch 'origin/main'",
          "timestamp": "2026-09-03T22:30:48+02:00",
          "tree_id": "35f77fb56a1fec3e611a92734405ef300d2e073d",
          "url": "https://github.com/maebli/m-bus-parser/commit/b435ea42e3fe33211d021db5ea3d2f80fca9f168"
        },
        "date": 1788467535935,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Eager full wired-frame parse stack",
            "value": 2784,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16; fixture=520 B; max(setup=476 B,records=2264 B)"
          },
          {
            "name": "Frame and application setup nested stack",
            "value": 476,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16; mbus=248 B; wired=64 B; application=228 B"
          },
          {
            "name": "Record decode nested stack",
            "value": 2264,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "Eager full-frame fixture local stack frame",
            "value": 520,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecords::next local stack frame",
            "value": 704,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecord::try_from local stack frame",
            "value": 16,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecord::parse local stack frame",
            "value": 456,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecordHeader::try_from local stack frame",
            "value": 280,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "ProcessedDataRecordHeader::try_from local stack frame",
            "value": 584,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "ValueInformation::try_from local stack frame",
            "value": 160,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "consume_orthhogonal_vife local stack frame",
            "value": 64,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "VIF block parser local stack frame",
            "value": 48,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecord value size",
            "value": 216,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "VIF block value size",
            "value": 20,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "Linked eager full parser text + data size",
            "value": 18887,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16; profile=opt-level=z,lto=fat,codegen-units=1; sections=text+data"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Michael Aebli",
            "username": "maebli"
          },
          "committer": {
            "name": "Michael Aebli",
            "username": "maebli"
          },
          "distinct": true,
          "id": "157cfeb96975f22e05ae2fd3f49b51b6b674158b",
          "message": "adding info for cortex-m demo",
          "timestamp": "2026-09-03T22:53:47+02:00",
          "tree_id": "dfda905fa6e532d52ba324a3b89da8aee2fe45f8",
          "url": "https://github.com/maebli/m-bus-parser/commit/157cfeb96975f22e05ae2fd3f49b51b6b674158b"
        },
        "date": 1788468914991,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Eager full wired-frame parse stack",
            "value": 2784,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16; fixture=520 B; max(setup=476 B,records=2264 B)"
          },
          {
            "name": "Frame and application setup nested stack",
            "value": 476,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16; mbus=248 B; wired=64 B; application=228 B"
          },
          {
            "name": "Record decode nested stack",
            "value": 2264,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "Eager full-frame fixture local stack frame",
            "value": 520,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecords::next local stack frame",
            "value": 704,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecord::try_from local stack frame",
            "value": 16,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecord::parse local stack frame",
            "value": 456,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecordHeader::try_from local stack frame",
            "value": 280,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "ProcessedDataRecordHeader::try_from local stack frame",
            "value": 584,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "ValueInformation::try_from local stack frame",
            "value": 160,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "consume_orthhogonal_vife local stack frame",
            "value": 64,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "VIF block parser local stack frame",
            "value": 48,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecord value size",
            "value": 216,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "VIF block value size",
            "value": 20,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "Linked eager full parser text + data size",
            "value": 18887,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16; profile=opt-level=z,lto=fat,codegen-units=1; sections=text+data"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Michael Aebli",
            "username": "maebli"
          },
          "committer": {
            "name": "Michael Aebli",
            "username": "maebli"
          },
          "distinct": true,
          "id": "de6d57291c139aa9d254fe128d07d0167d09c993",
          "message": "cleaning up cortex-m example",
          "timestamp": "2026-09-03T23:00:17+02:00",
          "tree_id": "021379cd504fd0537eebf1fc7d466db2b3a8d0af",
          "url": "https://github.com/maebli/m-bus-parser/commit/de6d57291c139aa9d254fe128d07d0167d09c993"
        },
        "date": 1788469292798,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Eager full wired-frame parse stack",
            "value": 2784,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16; fixture=520 B; max(setup=476 B,records=2264 B)"
          },
          {
            "name": "Frame and application setup nested stack",
            "value": 476,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16; mbus=248 B; wired=64 B; application=228 B"
          },
          {
            "name": "Record decode nested stack",
            "value": 2264,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "Eager full-frame fixture local stack frame",
            "value": 520,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecords::next local stack frame",
            "value": 704,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecord::try_from local stack frame",
            "value": 16,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecord::parse local stack frame",
            "value": 456,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecordHeader::try_from local stack frame",
            "value": 280,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "ProcessedDataRecordHeader::try_from local stack frame",
            "value": 584,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "ValueInformation::try_from local stack frame",
            "value": 160,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "consume_orthhogonal_vife local stack frame",
            "value": 64,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "VIF block parser local stack frame",
            "value": 48,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecord value size",
            "value": 216,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "VIF block value size",
            "value": 20,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "Linked eager full parser text + data size",
            "value": 18887,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16; profile=opt-level=z,lto=fat,codegen-units=1; sections=text+data"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "doland@duck.com",
            "name": "Michael Aebli",
            "username": "maebli"
          },
          "committer": {
            "email": "doland@duck.com",
            "name": "Michael Aebli",
            "username": "maebli"
          },
          "distinct": true,
          "id": "7d31bde172935729e3f58f1da5aa56cde03a9801",
          "message": "sanitize email",
          "timestamp": "2026-09-04T09:02:05+02:00",
          "tree_id": "06b9db2c920cba4b7410228329867dcd670811d5",
          "url": "https://github.com/maebli/m-bus-parser/commit/7d31bde172935729e3f58f1da5aa56cde03a9801"
        },
        "date": 1788505402930,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Eager full wired-frame parse stack",
            "value": 2784,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16; fixture=520 B; max(setup=476 B,records=2264 B)"
          },
          {
            "name": "Frame and application setup nested stack",
            "value": 476,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16; mbus=248 B; wired=64 B; application=228 B"
          },
          {
            "name": "Record decode nested stack",
            "value": 2264,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "Eager full-frame fixture local stack frame",
            "value": 520,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecords::next local stack frame",
            "value": 704,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecord::try_from local stack frame",
            "value": 16,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecord::parse local stack frame",
            "value": 456,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecordHeader::try_from local stack frame",
            "value": 280,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "ProcessedDataRecordHeader::try_from local stack frame",
            "value": 584,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "ValueInformation::try_from local stack frame",
            "value": 160,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "consume_orthhogonal_vife local stack frame",
            "value": 64,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "VIF block parser local stack frame",
            "value": 48,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "DataRecord value size",
            "value": 216,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "VIF block value size",
            "value": 20,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16"
          },
          {
            "name": "Linked eager full parser text + data size",
            "value": 18887,
            "unit": "bytes",
            "extra": "target=thumbv7em-none-eabi; toolchain=nightly-2026-05-16; profile=opt-level=z,lto=fat,codegen-units=1; sections=text+data"
          }
        ]
      }
    ],
    "Parser decode speed": [
      {
        "commit": {
          "author": {
            "name": "Maebli",
            "username": "maebli"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7fb01ef5909be1dc3e1856989315902bc7413ba1",
          "message": "Merge pull request #116 from maebli/feature/zero-copy-completion\n\nFeature/zero copy completion (still a bit to go but one step closer)",
          "timestamp": "2026-09-01T22:59:52+02:00",
          "tree_id": "c84ab0e35dd146128d98af67acdeb56579a255fe",
          "url": "https://github.com/maebli/m-bus-parser/commit/7fb01ef5909be1dc3e1856989315902bc7413ba1"
        },
        "date": 1788296468990,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_full_frame_eager",
            "value": 1166,
            "range": "± 25",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Michael Aebli",
            "username": "maebli"
          },
          "committer": {
            "name": "Michael Aebli",
            "username": "maebli"
          },
          "distinct": true,
          "id": "893b8f4c35bf52e482654aab31010274ad67dabb",
          "message": "Fixing pipeline for benches",
          "timestamp": "2026-09-03T21:54:36+02:00",
          "tree_id": "3ef63fddae036ac7a9353d9741fde7d14e8aecfd",
          "url": "https://github.com/maebli/m-bus-parser/commit/893b8f4c35bf52e482654aab31010274ad67dabb"
        },
        "date": 1788465372267,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_full_frame_eager",
            "value": 1318,
            "range": "± 13",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Michael Aebli",
            "username": "maebli"
          },
          "committer": {
            "name": "Michael Aebli",
            "username": "maebli"
          },
          "distinct": true,
          "id": "1364f2041d6373cfa8b39c3566e58f10efc8bf8a",
          "message": "adjusting examples",
          "timestamp": "2026-09-03T22:14:51+02:00",
          "tree_id": "88eb8753c99beb0e3ff0697907a1e77317eeeb2e",
          "url": "https://github.com/maebli/m-bus-parser/commit/1364f2041d6373cfa8b39c3566e58f10efc8bf8a"
        },
        "date": 1788466569035,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_full_frame_eager",
            "value": 1223,
            "range": "± 28",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Michael Aebli",
            "username": "maebli"
          },
          "committer": {
            "name": "Michael Aebli",
            "username": "maebli"
          },
          "distinct": true,
          "id": "b435ea42e3fe33211d021db5ea3d2f80fca9f168",
          "message": "Merge remote-tracking branch 'origin/main'",
          "timestamp": "2026-09-03T22:30:48+02:00",
          "tree_id": "35f77fb56a1fec3e611a92734405ef300d2e073d",
          "url": "https://github.com/maebli/m-bus-parser/commit/b435ea42e3fe33211d021db5ea3d2f80fca9f168"
        },
        "date": 1788467538049,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_full_frame_eager",
            "value": 953,
            "range": "± 7",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Michael Aebli",
            "username": "maebli"
          },
          "committer": {
            "name": "Michael Aebli",
            "username": "maebli"
          },
          "distinct": true,
          "id": "157cfeb96975f22e05ae2fd3f49b51b6b674158b",
          "message": "adding info for cortex-m demo",
          "timestamp": "2026-09-03T22:53:47+02:00",
          "tree_id": "dfda905fa6e532d52ba324a3b89da8aee2fe45f8",
          "url": "https://github.com/maebli/m-bus-parser/commit/157cfeb96975f22e05ae2fd3f49b51b6b674158b"
        },
        "date": 1788468917140,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_full_frame_eager",
            "value": 880,
            "range": "± 17",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Michael Aebli",
            "username": "maebli"
          },
          "committer": {
            "name": "Michael Aebli",
            "username": "maebli"
          },
          "distinct": true,
          "id": "de6d57291c139aa9d254fe128d07d0167d09c993",
          "message": "cleaning up cortex-m example",
          "timestamp": "2026-09-03T23:00:17+02:00",
          "tree_id": "021379cd504fd0537eebf1fc7d466db2b3a8d0af",
          "url": "https://github.com/maebli/m-bus-parser/commit/de6d57291c139aa9d254fe128d07d0167d09c993"
        },
        "date": 1788469294524,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_full_frame_eager",
            "value": 1316,
            "range": "± 12",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "doland@duck.com",
            "name": "Michael Aebli",
            "username": "maebli"
          },
          "committer": {
            "email": "doland@duck.com",
            "name": "Michael Aebli",
            "username": "maebli"
          },
          "distinct": true,
          "id": "7d31bde172935729e3f58f1da5aa56cde03a9801",
          "message": "sanitize email",
          "timestamp": "2026-09-04T09:02:05+02:00",
          "tree_id": "06b9db2c920cba4b7410228329867dcd670811d5",
          "url": "https://github.com/maebli/m-bus-parser/commit/7d31bde172935729e3f58f1da5aa56cde03a9801"
        },
        "date": 1788505404758,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_full_frame_eager",
            "value": 1317,
            "range": "± 14",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}