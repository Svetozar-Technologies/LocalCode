#!/usr/bin/env python3
"""Generate professional terminal mockup screenshots as SVG files."""

import html

TERMINAL_TEMPLATE = """<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">
  <defs>
    <style>
      .terminal-bg {{ fill: #1a1b26; }}
      .titlebar {{ fill: #24283b; }}
      .text {{ font-family: 'Menlo', 'Monaco', 'Courier New', monospace; font-size: 13px; }}
      .prompt {{ fill: #7aa2f7; }}
      .command {{ fill: #c0caf5; }}
      .success {{ fill: #9ece6a; }}
      .error {{ fill: #f7768e; }}
      .dim {{ fill: #565f89; }}
      .warning {{ fill: #e0af68; }}
      .cyan {{ fill: #7dcfff; }}
      .magenta {{ fill: #bb9af7; }}
      .white {{ fill: #c0caf5; }}
      .bold {{ font-weight: 700; }}
      .green {{ fill: #9ece6a; }}
      .yellow {{ fill: #e0af68; }}
      .blue {{ fill: #7aa2f7; }}
      .red {{ fill: #f7768e; }}
    </style>
    <filter id="shadow" x="-5%" y="-5%" width="110%" height="115%">
      <feDropShadow dx="0" dy="8" stdDeviation="12" flood-color="#000" flood-opacity="0.4"/>
    </filter>
  </defs>

  <!-- Window shadow -->
  <rect x="16" y="16" width="{inner_w}" height="{inner_h}" rx="12" fill="none" filter="url(#shadow)"/>

  <!-- Terminal background -->
  <rect x="16" y="16" width="{inner_w}" height="{inner_h}" rx="12" class="terminal-bg"/>

  <!-- Title bar -->
  <rect x="16" y="16" width="{inner_w}" height="40" rx="12" class="titlebar"/>
  <rect x="16" y="44" width="{inner_w}" height="12" class="titlebar"/>

  <!-- Traffic lights -->
  <circle cx="40" cy="36" r="7" fill="#ff5f57"/>
  <circle cx="62" cy="36" r="7" fill="#febc2e"/>
  <circle cx="84" cy="36" r="7" fill="#28c840"/>

  <!-- Title -->
  <text x="{title_x}" y="41" class="text dim" text-anchor="middle" font-size="12">{title}</text>

  <!-- Content -->
  <g transform="translate(36, 76)">
{lines}
  </g>
</svg>"""


def make_line(y, spans):
    """Create a line of text with multiple styled spans."""
    parts = []
    x = 0
    for text, css_class in spans:
        escaped = html.escape(text)
        if css_class:
            parts.append(f'    <text x="{x}" y="{y}" class="text {css_class}">{escaped}</text>')
        else:
            parts.append(f'    <text x="{x}" y="{y}" class="text white">{escaped}</text>')
        x += len(text) * 7.8  # approximate monospace char width
    return "\n".join(parts)


def make_svg(title, content_lines, width=750, line_height=20):
    """Generate full SVG from lines."""
    lines_svg = []
    for i, spans in enumerate(content_lines):
        lines_svg.append(make_line(i * line_height, spans))

    content_height = len(content_lines) * line_height + 40
    height = content_height + 76 + 32  # title bar + padding
    inner_w = width - 32
    inner_h = height - 32

    return TERMINAL_TEMPLATE.format(
        width=width,
        height=height,
        inner_w=inner_w,
        inner_h=inner_h,
        title_x=width // 2,
        title=html.escape(title),
        lines="\n".join(lines_svg),
    )


# ============================================================
# Screenshot 1: Self-Healing Agent in Action
# ============================================================
self_healing_lines = [
    [("$ ", "prompt"), ("localcode-cli", "command")],
    [("", "")],
    [("  LocalCode v0.2.0", "cyan"), ("  --  Privacy-first AI code editor", "dim")],
    [("", "")],
    [("You: ", "dim"), ("create a python script that fetches bitcoin price and run it", "white")],
    [("", "")],
    [("  * ", "blue"), ("Thinking...", "dim")],
    [("  > ", "magenta"), ("write_file", "cyan"), ("  bitcoin_price.py", "dim")],
    [("      + ", "success"), ("Wrote 18 lines to bitcoin_price.py", "dim")],
    [("  > ", "magenta"), ("run_command", "cyan"), ("  python3 bitcoin_price.py", "dim")],
    [("      X ", "error"), ("[ERROR exit_code=1]", "error")],
    [("        ", ""), ("ModuleNotFoundError: No module named 'requests'", "error")],
    [("", "")],
    [("  * ", "blue"), ("Auto-fixing... (attempt 1/3)", "warning")],
    [("  > ", "magenta"), ("run_command", "cyan"), ("  pip install requests", "dim")],
    [("      + ", "success"), ("Successfully installed requests-2.31.0", "dim")],
    [("  > ", "magenta"), ("run_command", "cyan"), ("  python3 bitcoin_price.py", "dim")],
    [("      + ", "success"), ("Bitcoin: $67,432.18 USD", "success")],
    [("", "")],
    [("  Agent: ", "cyan"), ("Done! The script fetches the current Bitcoin price", "white")],
    [("  ", ""), ("from CoinGecko API. Current price: $67,432.18 USD", "white")],
]

# ============================================================
# Screenshot 2: Session Memory & Search
# ============================================================
sessions_lines = [
    [("You: ", "dim"), ("/sessions", "cyan")],
    [("", "")],
    [("  Recent Sessions", "cyan bold")],
    [("  -----------------------------------------------------------", "dim")],
    [("  ", ""), ("a3f8c2", "yellow"), ("  2026-03-19 14:32  ", "dim"), ("LocalCode    ", "blue"), ("Self-healing agent implementation", "white")],
    [("  ", ""), ("b7d1e5", "yellow"), ("  2026-03-18 09:15  ", "dim"), ("WebApp       ", "blue"), ("React auth system with JWT", "white")],
    [("  ", ""), ("c9a4f0", "yellow"), ("  2026-03-17 16:45  ", "dim"), ("PythonML     ", "blue"), ("Snake game with pygame", "white")],
    [("  ", ""), ("d2b6c8", "yellow"), ("  2026-03-16 11:20  ", "dim"), ("DataPipeline ", "blue"), ("CSV parser with error handling", "white")],
    [("  ", ""), ("e5f3a1", "yellow"), ("  2026-03-15 08:30  ", "dim"), ("API-Server   ", "blue"), ("REST API with rate limiting", "white")],
    [("  -----------------------------------------------------------", "dim")],
    [("  ", ""), ("5 sessions", "dim"), (" across ", "dim"), ("5 projects", "dim")],
    [("", "")],
    [("", "")],
    [("You: ", "dim"), ("/sessions search snake game", "cyan")],
    [("", "")],
    [("  Search Results", "cyan bold"), (" for \"snake game\"", "dim")],
    [("  -----------------------------------------------------------", "dim")],
    [("  ", ""), ("* ", "green"), ("c9a4f0", "yellow"), ("  ", ""), ("Snake game with pygame", "white"), ("  [0.94]", "green")],
    [("    ", ""), ("PythonML  |  2026-03-17  |  python, pygame, game", "dim")],
    [("  -----------------------------------------------------------", "dim")],
    [("", "")],
    [("", "")],
    [("You: ", "dim"), ("/recall snake game", "cyan")],
    [("", "")],
    [("  OK ", "success"), ("Loaded session context: Snake game with pygame", "green")],
    [("  ", ""), ("Files: snake.py, assets/  |  Tags: python, pygame, game", "dim")],
]

# ============================================================
# Screenshot 3: Error Recovery Flow (detailed)
# ============================================================
error_recovery_lines = [
    [("You: ", "dim"), ("build a web scraper for HN top stories and run it", "white")],
    [("", "")],
    [("  > ", "magenta"), ("write_file", "cyan"), ("  hn_scraper.py", "dim")],
    [("      + ", "success"), ("Wrote 24 lines", "dim")],
    [("  > ", "magenta"), ("run_command", "cyan"), ("  python3 hn_scraper.py", "dim")],
    [("      X ", "error"), ("[ERROR exit_code=1]", "error")],
    [("        ", ""), ("NameError: name 'soup' is not defined (line 12)", "error")],
    [("", "")],
    [("  > ", "magenta"), ("read_file", "cyan"), ("  hn_scraper.py (checking line 12)", "dim")],
    [("  > ", "magenta"), ("edit_file", "cyan"), ("  Fixed: added soup = BeautifulSoup(resp.text)", "dim")],
    [("      + ", "success"), ("Applied 1 edit", "dim")],
    [("  > ", "magenta"), ("run_command", "cyan"), ("  python3 hn_scraper.py", "dim")],
    [("      + ", "success"), ("Top 5 Hacker News Stories:", "success")],
    [("        ", ""), ("1. Show HN: I built an AI that debugs itself  (342 pts)", "white")],
    [("        ", ""), ("2. Rust is eating the world                    (289 pts)", "white")],
    [("        ", ""), ("3. Why local LLMs are the future               (256 pts)", "white")],
    [("        ", ""), ("4. The death of SaaS subscriptions             (198 pts)", "white")],
    [("        ", ""), ("5. Open source AI tools you should know        (187 pts)", "white")],
    [("", "")],
    [("  Agent: ", "cyan"), ("Done! Scraped top 5 stories from Hacker News.", "white")],
    [("  ", ""), ("Fixed a NameError (missing BeautifulSoup init) on first try.", "dim")],
]

# ============================================================
# Screenshot 4: Quick Start / First Run
# ============================================================
quickstart_lines = [
    [("$ ", "prompt"), ("git clone https://github.com/Svetozar-Technologies/LocalCode.git", "command")],
    [("  Cloning into 'LocalCode'...", "dim")],
    [("$ ", "prompt"), ("cd LocalCode && cargo build --release", "command")],
    [("  ", ""), ("Compiling localcode-cli v0.2.0", "dim")],
    [("  ", ""), ("Finished `release` in 45s", "success")],
    [("$ ", "prompt"), ("./target/release/localcode-cli", "command")],
    [("", "")],
    [("", "")],
    [("  +==================================================+", "cyan")],
    [("  |", "cyan"), ("         LocalCode v0.2.0                       ", "white"), ("|", "cyan")],
    [("  |", "cyan"), ("     Privacy-first AI code editor                ", "dim"), ("|", "cyan")],
    [("  +==================================================+", "cyan")],
    [("", "")],
    [("  ", ""), ("/model ollama:llama3", "yellow"), ("     <- Use local model (free, private)", "dim")],
    [("  ", ""), ("/model openai:gpt-4o", "yellow"), ("     <- Use OpenAI API", "dim")],
    [("  ", ""), ("/model anthropic:claude", "yellow"), ("  <- Use Anthropic API", "dim")],
    [("", "")],
    [("  Commands: ", "dim"), ("/help", "cyan"), (" /sessions", "cyan"), (" /recall", "cyan"), (" /memory", "cyan"), (" /commit", "cyan")],
    [("", "")],
    [("You: ", "dim"), ("_", "white")],
]


# ============================================================
# Generate all SVGs + PNGs
# ============================================================
screenshots = {
    "1-self-healing-agent": ("LocalCode -- Self-Healing Agent", self_healing_lines, 680),
    "2-session-memory": ("LocalCode -- Session Memory & Search", sessions_lines, 700),
    "3-error-recovery": ("LocalCode -- Auto Error Recovery", error_recovery_lines, 700),
    "4-quick-start": ("LocalCode -- Quick Start", quickstart_lines, 700),
}

for filename, (title, lines, width) in screenshots.items():
    svg = make_svg(title, lines, width=width)
    svg_path = f"/Users/prepladder/LocalCode/assets/screenshots/{filename}.svg"
    png_path = f"/Users/prepladder/LocalCode/assets/screenshots/{filename}.png"
    with open(svg_path, "w") as f:
        f.write(svg)
    print(f"  SVG: {svg_path}")

    # Convert to PNG
    try:
        import cairosvg
        cairosvg.svg2png(url=svg_path, write_to=png_path, scale=2)
        print(f"  PNG: {png_path}")
    except Exception as e:
        print(f"  PNG failed: {e}")

print(f"\nGenerated {len(screenshots)} screenshots")
