window.BENCHMARK_DATA = {
  "lastUpdate": 1788465372935,
  "repoUrl": "https://github.com/maebli/m-bus-parser",
  "entries": {
    "Parser stack and footprint": [
      {
        "commit": {
          "author": {
            "email": "1138612+maebli@users.noreply.github.com",
            "name": "Maebli",
            "username": "maebli"
          },
          "committer": {
            "email": "noreply@github.com",
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
      }
    ],
    "Parser decode speed": [
      {
        "commit": {
          "author": {
            "email": "1138612+maebli@users.noreply.github.com",
            "name": "Maebli",
            "username": "maebli"
          },
          "committer": {
            "email": "noreply@github.com",
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
      }
    ]
  }
}