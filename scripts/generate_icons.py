import os
from playwright.sync_api import sync_playwright
from PIL import Image

def generate_icons():
    base_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
    icon_svg_path = os.path.join(base_dir, "featherwall_icon.svg")
    logo_svg_path = os.path.join(base_dir, "featherwall_logo.svg")
    assets_dir = os.path.join(base_dir, "assets")
    os.makedirs(assets_dir, exist_ok=True)

    with open(icon_svg_path, "r", encoding="utf-8") as f:
        icon_svg = f.read()

    with open(logo_svg_path, "r", encoding="utf-8") as f:
        logo_svg = f.read()

    with sync_playwright() as p:
        browser = p.chromium.launch()

        # Render Icon 512x512
        page = browser.new_page(viewport={"width": 512, "height": 512}, device_scale_factor=1)
        html_icon = f"""<!DOCTYPE html>
<html>
<head>
<style>
  html, body {{
    margin: 0; padding: 0; width: 512px; height: 512px; background: transparent; overflow: hidden;
  }}
  svg {{
    width: 512px; height: 512px; display: block;
  }}
</style>
</head>
<body>
{icon_svg}
</body>
</html>"""
        page.set_content(html_icon)
        icon_png_path = os.path.join(assets_dir, "icon_512.png")
        page.screenshot(path=icon_png_path, omit_background=True)
        print(f"Rendered {icon_png_path}")

        # Render Logo 1050x740
        page2 = browser.new_page(viewport={"width": 1050, "height": 740}, device_scale_factor=1)
        html_logo = f"""<!DOCTYPE html>
<html>
<head>
<style>
  html, body {{
    margin: 0; padding: 0; width: 1050px; height: 740px; background: transparent; overflow: hidden;
  }}
  svg {{
    width: 1050px; height: 740px; display: block;
  }}
</style>
</head>
<body>
{logo_svg}
</body>
</html>"""
        page2.set_content(html_logo)
        logo_png_path = os.path.join(assets_dir, "logo.png")
        page2.screenshot(path=logo_png_path, omit_background=True)
        print(f"Rendered {logo_png_path}")

        browser.close()

    # Create ICO file with PIL
    img = Image.open(icon_png_path).convert("RGBA")
    
    # Save standard icon.png for docs/readme
    img.save(os.path.join(assets_dir, "icon.png"), "PNG")
    
    # Save multi-resolution .ico for Windows Executable
    ico_path = os.path.join(assets_dir, "featherwall.ico")
    icon_sizes = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    img.save(ico_path, format="ICO", sizes=icon_sizes)
    print(f"Generated multi-resolution ICO at {ico_path}")

if __name__ == "__main__":
    generate_icons()
