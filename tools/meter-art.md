# Meter artwork

The project distributes its meter drawing code and the WebPs rendered from it under the [MIT license](../LICENSE). The manufacturer references supply physical facts. The renderer reads geometry, drawing instructions and decoded telegram fields. The export blocks all image requests, including requests for existing fallbacks and thumbnails.

## Rebuild from the repository root

Install Python 3, Pillow, Playwright and a Chromium browser. These tools are needed only to export or compare artwork.

```sh
python3 -m venv .venv
.venv/bin/pip install Pillow==12.3.0 playwright==1.60.0
.venv/bin/playwright install chromium
.venv/bin/python tools/render-meter-art.py
.venv/bin/python tools/render-meter-art.py --check
```

Pass `--chromium /path/to/chromium` to select an installed browser. The checked-in export used Chromium 137.0.7111.0 on macOS, Pillow 12.3.0 and libwebp 1.6.0. A byte comparison requires matching browser, fonts and encoder. For a toolchain change, review the regenerated images visually.

The script opens the local parser, selects each catalogue example and captures its initial camera angle. A shared thumbnail uses the first example with that filename. The mode 13 Elster thumbnail uses the bundled TLS example. The script draws a second set with empty readings for the static fallbacks, including the six generic device families. It crops transparent margins and scales the longest side to at most 120 pixels for thumbnails or 512 for fallbacks. The check also reports missing and surplus WebPs.

## Drawing sources

Look up the filename stem in [the model catalogue](../docs/products/catalog.js) for the manufacturer, model and reference. Shared housings use [model profiles](../docs/products/profiles.js), [solid geometry](../docs/products/housing.js) and [face drawings](../docs/products/faces.js). Profile sizes are nominal or approximate in millimetres. Screen and button positions are fractions of the front panel. Drawings represent one mounting arrangement and colour variant per model.

| Published stem or family | Drawing and physical reference |
| --- | --- |
| `gwf-mtkcoder` | Brass threaded body, open blue lid, roller window and encoder bar. The horizontal DN20 reference has a 220 mm body and litre-resolution register. [Geometry](../docs/products/geometry.js), [dial](../docs/products/water.js), [GWF data sheet](https://productfinder.gwf.ch/en/export?id=4062) |
| `multical-601` | Nominal 147 × 100 × 52 mm calculator with a left LCD, two vertically stacked keys and a ribbed right panel. [Profile](../docs/products/profiles.js), [Kamstrup dimensional sketches](https://kams-p-001.sitecorecontenthub.cloud/api/public/content/59557-downloadOriginal?v=d16a7c2f) |
| ABB DELTA, Berg DZ+, Eastron SDM630, EMH DIZ, EMU Professional 375, Finder 7E, NZR DHZ and Saia ALE3 | Stepped DIN-rail cases, terminal rows and front registers. ABB and Berg share the DZ+/DELTAplus case. Examples include the 17.5 mm Finder, the 72 mm SDM630 and the ALE3 with blue covers and a backlit LCD. [Model sheets](../docs/products/catalog.js) |
| Allmess CF50, Elster F2, Engelmann SensoStar 2C, Itron CF51 and CF55, Kamstrup 382, Minol WR3, Sensus PolluTherm, Sontex Supercal 531 and SVM F22 | Wall calculators and enclosures with model-specific displays, controls, cable entries and probe leads. The CF50 front follows drawing 6318-1. The CF51 reference measures 149 × 119 × 55 mm. [Model sheets](../docs/products/catalog.js), [CF51 dimensions](https://na.itron.com/o/commerce-media/accounts/-1/attachments/3816337), [382 dimensions](https://kamstrup-delivery.sitecorecontenthub.cloud/api/public/content/61121-downloadOriginal?v=dcbc5f67) |
| Engelmann SensoStar 2 and WaterStar, Itron CF Echo II and Integral MK MaXX, ULTRAHEAT T230 and XS, Minocal C2, PolluCom E, PolluStat and PolluStat E, Siemens WFH21 | Flow bodies, flanges, probes and calculator heads assembled from solids. Reference variants include the flanged CF Echo II, the dark composite T230 body and the round WaterStar M register. [Model sheets](../docs/products/catalog.js), [WaterStar family reference](https://www.engelmann.de/en/product/waterstar-m/) |
| Elster Falcon, Itron BM +m and Cyble, Relay PadPuls 2, WEP room sensor | Cable pickups, optical window, enclosure knockouts and vents. Screenless devices show readings beside the housing. WEP uses a family illustration because the fixture identifies a room sensor. [Model references](../docs/products/catalog.js) |
| GMC A230 with EMMOD206, Sensus iPERL, Landis+Gyr G350 and Elster gas | Existing project drawings with simplified unbranded faces. The gas fixtures use the E6 single-pipe and BK-G4 reference housings. [A230](../docs/products/a230.js), [iPERL](../docs/products/iperl.js), [G350](../docs/products/g350.js), [Elster](../docs/products/elster.js) |
| All six `generic-*` assets | Project illustrations for devices with limited identification. The generic water register uses the existing hinged lid and mechanical dial. [Water register](../docs/products/water.js), [geometry](../docs/products/geometry.js), [profiles](../docs/products/profiles.js) |

## Asset audit

The previous catalogue included manufacturer links, but the repository did not document permission to redistribute the GWF and MULTICAL bitmaps or the other externally derived meter faces. All 46 fallback WebPs and all 42 thumbnail WebPs were regenerated. The earlier electricity, heat and water catalogue tiles were removed. The report flag is a separate interface icon with its path in the [SVG source](../docs/products/report-flag.svg).
