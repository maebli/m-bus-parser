import argparse
import base64
import functools
import io
import re
import threading
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

from PIL import Image
from playwright.sync_api import sync_playwright

DOCS = Path(__file__).resolve().parents[1] / "docs"
PRODUCTS = DOCS / "products"
STAGE = "meter-turntable"


class SiteFiles(SimpleHTTPRequestHandler):
    def log_message(self, *args):
        pass


def snapshot(stage, max_size):
    stage.evaluate("(stage) => stage.draw()")
    url = stage.evaluate("(stage) => stage.canvas.toDataURL()")
    image = Image.open(io.BytesIO(base64.b64decode(url.split(",", 1)[1])))
    bounds = image.getbbox()
    if not bounds:
        raise ValueError("The meter render is empty")
    image = image.crop(bounds)
    image.thumbnail((max_size, max_size), Image.Resampling.LANCZOS)
    output = io.BytesIO()
    image.save(output, format="WEBP", quality=92, method=6, exact=True)
    return output.getvalue()


def render(page, site):
    page.goto(site + "/?example=multical-601&format=product")
    page.wait_for_function("document.querySelector('meter-turntable')?.geometry")
    samples = page.locator(".meter-sample")
    previews = {}
    assets = set()
    for sample in samples.all():
        filename = sample.locator("img").get_attribute("src").split("/")[-1]
        if not re.fullmatch(r"[a-z0-9-]+\.webp", filename):
            raise ValueError(f"Unexpected catalogue filename {filename}")
        sample.evaluate("(sample) => sample.click()")
        stage = page.locator(STAGE)
        asset = stage.evaluate("(stage) => stage.asset")
        if asset != filename.removesuffix(".webp").removesuffix("-mode-13"):
            raise ValueError(f"{filename} opens {asset}")
        if filename not in previews:
            previews[filename] = snapshot(stage, 120)
        assets.add(asset)

    files = {PRODUCTS / "previews" / name: data for name, data in previews.items()}
    families = ["water", "gas", "heat", "electricity", "sensor", "device"]
    assets.update("generic-" + family for family in families)
    for asset in sorted(assets):
        stage = page.locator(STAGE)
        stage.evaluate(
            """(old, asset) => {
            const stage = document.createElement('meter-turntable');
            stage.configure(asset, asset, null, []);
            old.replaceWith(stage);
        }""",
            asset,
        )
        files[PRODUCTS / (asset + ".webp")] = snapshot(page.locator(STAGE), 512)
    return files


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--chromium", help="Path to a Chromium executable")
    parser.add_argument("--check", action="store_true", help="Compare without writing")
    args = parser.parse_args()
    handler = functools.partial(SiteFiles, directory=DOCS)
    with (
        ThreadingHTTPServer(("127.0.0.1", 0), handler) as server,
        sync_playwright() as runner,
    ):
        threading.Thread(target=server.serve_forever, daemon=True).start()
        site = f"http://127.0.0.1:{server.server_port}"
        browser = runner.chromium.launch(executable_path=args.chromium)
        page = browser.new_page(
            viewport={"width": 940, "height": 950}, device_scale_factor=1
        )
        page.route(
            "**/*",
            lambda route: (
                route.continue_()
                if route.request.url.startswith(site + "/")
                and route.request.resource_type != "image"
                else route.abort()
            ),
        )
        files = render(page, site)
        browser.close()
        server.shutdown()

    if args.check:
        published = set(PRODUCTS.glob("*.webp")) | set(
            (PRODUCTS / "previews").glob("*.webp")
        )
        stale = published.symmetric_difference(files)
        stale.update(
            path
            for path in published.intersection(files)
            if path.read_bytes() != files[path]
        )
        if stale:
            raise SystemExit(
                "Stale meter artwork: "
                + ", ".join(sorted(str(p.relative_to(DOCS)) for p in stale))
            )
    else:
        for path, data in files.items():
            path.parent.mkdir(exist_ok=True)
            path.write_bytes(data)
    print(f"{'Verified' if args.check else 'Rendered'} {len(files)} meter images")


if __name__ == "__main__":
    main()
