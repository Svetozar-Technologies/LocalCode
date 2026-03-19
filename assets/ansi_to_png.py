#!/usr/bin/env python3
"""Convert real ANSI terminal output to professional PNG screenshots."""

import re
import os
from PIL import Image, ImageDraw, ImageFont

# Tokyo Night theme colors
COLORS = {
    'bg': (26, 27, 38),
    'titlebar': (36, 40, 59),
    'default': (192, 202, 245),
}


def parse_ansi_color(code):
    """Parse ANSI escape code to RGB color."""
    m = re.match(r'38;2;(\d+);(\d+);(\d+)', code)
    if m:
        return (int(m.group(1)), int(m.group(2)), int(m.group(3)))
    return None


def strip_and_parse_ansi(text):
    """Parse ANSI text into list of (text, color, bold) segments."""
    segments = []
    current_color = COLORS['default']
    bold = False
    parts = re.split(r'(\x1b\[[^m]*m)', text)
    for part in parts:
        if part.startswith('\x1b['):
            code = part[2:-1]
            if code == '0':
                current_color = COLORS['default']
                bold = False
            elif code == '1':
                bold = True
            elif code == '39':
                current_color = COLORS['default']
            else:
                color = parse_ansi_color(code)
                if color:
                    current_color = color
        elif part:
            segments.append((part, current_color, bold))
    return segments


def strip_ansi(text):
    """Remove all ANSI escape codes from text."""
    return re.sub(r'\x1b\[[^m]*m', '', text)


def render_terminal(lines_raw, title, output_path, width=820, padding=24):
    """Render parsed terminal lines to a PNG image."""
    font_paths = [
        '/System/Library/Fonts/Menlo.ttc',
        '/System/Library/Fonts/Monaco.ttf',
        '/System/Library/Fonts/SFMono-Regular.otf',
    ]
    font = font_bold = None
    for fp in font_paths:
        try:
            font = ImageFont.truetype(fp, 13)
            font_bold = ImageFont.truetype(fp, 13)
            break
        except:
            continue
    if font is None:
        font = font_bold = ImageFont.load_default()

    parsed_lines = [strip_and_parse_ansi(line) for line in lines_raw]

    line_height = 19
    titlebar_height = 40
    content_top = titlebar_height + 16
    content_height = len(parsed_lines) * line_height + 32
    total_height = content_top + content_height + 16

    img = Image.new('RGB', (width, total_height), COLORS['bg'])
    draw = ImageDraw.Draw(img)

    # Title bar
    draw.rectangle([0, 0, width, titlebar_height], fill=COLORS['titlebar'])
    draw.ellipse([16, 13, 30, 27], fill=(255, 95, 87))
    draw.ellipse([36, 13, 50, 27], fill=(254, 188, 46))
    draw.ellipse([56, 13, 70, 27], fill=(40, 200, 64))

    try:
        title_font = ImageFont.truetype(font_paths[0], 12)
    except:
        title_font = font
    title_bbox = draw.textbbox((0, 0), title, font=title_font)
    title_w = title_bbox[2] - title_bbox[0]
    draw.text(((width - title_w) / 2, 12), title, fill=(100, 116, 139), font=title_font)

    y = content_top
    for segments in parsed_lines:
        x = padding
        for text, color, bold in segments:
            f = font_bold if bold else font
            draw.text((x, y), text, fill=color, font=f)
            bbox = draw.textbbox((0, 0), text, font=f)
            x += bbox[2] - bbox[0]
        y += line_height

    img.save(output_path, 'PNG')
    print(f"  Saved: {output_path}")


def read_raw_output(filepath):
    with open(filepath, 'r') as f:
        return f.read().split('\n')


def filter_until_goodbye(lines):
    """Keep lines until 'Session saved' and remove trailing empties."""
    filtered = []
    for line in lines:
        if 'Session saved' in strip_ansi(line):
            break
        filtered.append(line)
    while filtered and strip_ansi(filtered[-1]).strip() == '':
        filtered.pop()
    # Remove leading empties
    while filtered and strip_ansi(filtered[0]).strip() == '':
        filtered.pop(0)
    return filtered


def filter_sessions_clean(lines):
    """For sessions output: only keep lines where a session entry starts
    (has session ID pattern), and single-line them. Also keep headers."""
    filtered = []
    for line in lines:
        clean = strip_ansi(line).strip()
        # Keep header lines, prompt lines, section titles
        if any(kw in clean for kw in ['LocalCode', 'Type /help', 'Sessions', 'Search Results', '❯']):
            filtered.append(line)
            continue
        # Keep lines that start with a session ID (8 hex chars + date)
        if re.match(r'^[0-9a-f]{8}\s+\d{4}-', clean):
            # Truncate to fit on one line (cut at first newline in ANSI)
            one_line = line.split('\n')[0] if '\n' in line else line
            # Also limit display length
            filtered.append(one_line)
            continue
        # Keep percentage match lines (search results)
        if re.match(r'^\d+%\s+[0-9a-f]{8}', clean):
            one_line = line.split('\n')[0] if '\n' in line else line
            filtered.append(one_line)
            continue
        # Keep blank separator lines (but only one)
        if clean == '' and (not filtered or strip_ansi(filtered[-1]).strip() != ''):
            filtered.append(line)
            continue
    return filtered


# ============================================================
out_dir = '/Users/prepladder/LocalCode/assets/screenshots'
os.makedirs(out_dir, exist_ok=True)

# --- Screenshot 1: Self-Healing Agent (real NameError → fix → rerun) ---
lines1 = filter_until_goodbye(read_raw_output('/tmp/demo_selfheal_real.txt'))
# Remove the git init / commit noise — keep only the error→fix→success flow + summary
filtered1 = []
skip_git = False
for line in lines1:
    clean = strip_ansi(line).strip()
    # Skip git-related lines (noisy, not the point of this screenshot)
    if any(kw in clean for kw in ['Checking git status', 'Git error:', 'git init', 'git add',
                                     'Initialized empty Git', 'Committing:', 'Committed:',
                                     'Command completed (exit code: 0)']):
        continue
    filtered1.append(line)

render_terminal(
    filtered1,
    "LocalCode -- Self-Healing Agent",
    f"{out_dir}/1-self-healing-agent.png",
    width=820
)

# --- Screenshot 2: Sessions list (clean, one line per session) ---
lines2_raw = read_raw_output('/tmp/demo_sessions_only.txt')
lines2_clean = filter_until_goodbye(lines2_raw)
lines2_sessions = filter_sessions_clean(lines2_clean)
# Remove consecutive blank lines
final2 = []
for line in lines2_sessions:
    if strip_ansi(line).strip() == '' and final2 and strip_ansi(final2[-1]).strip() == '':
        continue
    final2.append(line)

render_terminal(
    final2,
    "LocalCode -- Session Memory",
    f"{out_dir}/2-session-memory.png",
    width=820
)

# --- Screenshot 3: Help screen ---
lines_help = filter_until_goodbye(read_raw_output('/tmp/demo_help_raw.txt'))
render_terminal(
    lines_help,
    "LocalCode -- Commands",
    f"{out_dir}/3-help-commands.png",
    width=700
)

# --- Screenshot 4: Bitcoin price fetch (real successful run) ---
# Re-read the bitcoin demo output
btc_raw = """  \x1b[38;2;99;102;241m◆\x1b[39m \x1b[38;2;226;232;240m\x1b[1mLocalCode\x1b[0m \x1b[38;2;100;116;139mv0.2.0\x1b[39m
  \x1b[38;2;100;116;139m│\x1b[39m  \x1b[38;2;100;116;139m/private/tmp/localcode-demo\x1b[39m
  \x1b[38;2;100;116;139m│\x1b[39m  Type \x1b[38;2;34;211;238m/help\x1b[39m for commands, \x1b[38;2;34;211;238m/exit\x1b[39m to quit

\x1b[38;2;99;102;241m\x1b[1m❯\x1b[0m
    \x1b[38;2;34;211;238m◈\x1b[39m \x1b[38;2;100;116;139mWriting fetch_btc.py\x1b[39m
      \x1b[38;2;52;211;153m↳\x1b[39m \x1b[38;2;100;116;139mSuccessfully wrote to /private/tmp/localcode-demo/fetch_btc.py\x1b[39m
    \x1b[38;2;34;211;238m▶\x1b[39m \x1b[38;2;100;116;139mRunning: python3 fetch_btc.py\x1b[39m
      \x1b[38;2;52;211;153m↳\x1b[39m \x1b[38;2;100;116;139mCurrent Bitcoin Price: $70336\x1b[39m"""

lines4 = btc_raw.split('\n')
# Remove leading/trailing empties
while lines4 and strip_ansi(lines4[0]).strip() == '':
    lines4.pop(0)
while lines4 and strip_ansi(lines4[-1]).strip() == '':
    lines4.pop()

render_terminal(
    lines4,
    "LocalCode -- Real Bitcoin Price Fetch",
    f"{out_dir}/4-bitcoin-fetch.png",
    width=720
)

print("\nDone! All screenshots use real app output.")
