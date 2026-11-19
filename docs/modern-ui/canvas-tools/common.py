import math, os

OUT = os.path.join(os.path.dirname(__file__), '..', 'canvas', 'project')

TEXT = '#F4EBDC'
SUB = '#C8B89C'
MUTED = '#A89A82'
TRIM = 'rgba(201,160,98,0.8)'
TRIM_SOFT = 'rgba(201,160,98,0.35)'
ACCENT = '#E0B068'
PANEL = 'rgba(20,16,12,0.94)'
WORLD = 'linear-gradient(180deg, #8FB3C9 0%, #A9C4C2 40%, #6F8A55 100%)'

# retail radar colours
GOLD = '#FFAB00'      # attackable creatures
WHITE = '#FFFFFF'     # players, NPCs
GREEN = '#00FF00'     # fellowship
PURPLE = '#BF63FF'    # portals
YELLOW = '#FFFF80'    # vendors

# one shared scene: (x, y) as a fraction of the radius, player at 0,0 facing up
BLIPS = [
    dict(x=0.34, y=-0.46, shape='plus', color=GOLD, name='Drudge Skulker', dist='31 m', sel=True),
    dict(x=0.58, y=-0.18, shape='plus', color=GOLD, name='Drudge Sneaker', dist='45 m'),
    dict(x=-0.62, y=0.30, shape='plus', color=GOLD, name='Drudge Prowler', dist='52 m'),
    dict(x=-0.18, y=0.26, shape='tri_down', color=GREEN, name='Osk (fellow)', dist='24 m'),
    dict(x=-0.34, y=0.06, shape='tri', color=GREEN, name='Rook (fellowship leader)', dist='26 m'),
    dict(x=0.26, y=0.56, shape='box', color=WHITE, name='Mira (allegiance)', dist='46 m'),
    dict(x=-0.55, y=-0.52, shape='plus', color=YELLOW, name='Shopkeeper Renald', dist='57 m'),
    dict(x=0.08, y=-0.80, shape='plus', color=PURPLE, name='Portal to Glenden Wood', dist='60 m'),
    dict(x=0.72, y=0.40, shape='plus', color=WHITE, name='Tamsin', dist='62 m'),
]

HEADING = 42  # degrees, facing north-east


def blip_shape(shape, cx, cy, s, color, glow=False):
    f = f' filter="url(#glow)"' if glow else ''
    if shape == 'plus':
        return (f'<path d="M{cx-s} {cy}H{cx+s}M{cx} {cy-s}V{cy+s}" stroke="{color}" stroke-width="2.2" '
                f'stroke-linecap="round"{f}></path>')
    if shape == 'tri':
        return f'<path d="M{cx} {cy-s-1}L{cx+s+1} {cy+s}H{cx-s-1}z" fill="{color}"{f}></path>'
    if shape == 'tri_down':
        return f'<path d="M{cx} {cy+s+1}L{cx+s+1} {cy-s}H{cx-s-1}z" fill="{color}"{f}></path>'
    if shape == 'box':
        return (f'<rect x="{cx-s}" y="{cy-s}" width="{2*s}" height="{2*s}" fill="none" stroke="{color}" '
                f'stroke-width="2"{f}></rect>')
    if shape == 'x':
        return (f'<path d="M{cx-s} {cy-s}L{cx+s} {cy+s}M{cx+s} {cy-s}L{cx-s} {cy+s}" stroke="{color}" '
                f'stroke-width="2.2" stroke-linecap="round"{f}></path>')
    return ''


def blips_svg(r, s=4, glow=False, sel_ring=True, dots=False, c=None):
    out = []
    c = r if c is None else c
    for b in BLIPS:
        cx = round(c + b['x'] * r, 1)
        cy = round(c + b['y'] * r, 1)
        if dots:
            out.append(f'<circle cx="{cx}" cy="{cy}" r="3.5" fill="{b["color"]}"></circle>')
        else:
            out.append(blip_shape(b['shape'], cx, cy, s, b['color'], glow))
        if b.get('sel') and sel_ring:
            out.append(f'<circle cx="{cx}" cy="{cy}" r="{s+5}" fill="none" stroke="{ACCENT}" stroke-width="1.5"></circle>')
    return '\n'.join(out)


def player_arrow(r, size=7, color='#FFFFFF'):
    return (f'<path d="M{r} {r-size-2}L{r+size*0.8} {r+size}L{r} {r+size-4}L{r-size*0.8} {r+size}z" fill="{color}" '
            f'stroke="rgba(0,0,0,0.6)" stroke-width="1"></path>')


def compass_letters(r, inset, size=11, rotate=True):
    out = []
    for letter, ang in (('N', 0), ('E', 90), ('S', 180), ('W', 270)):
        a = math.radians(ang - (HEADING if rotate else 0))
        x = r + math.sin(a) * (r - inset)
        y = r - math.cos(a) * (r - inset)
        col = ACCENT if letter == 'N' else 'rgba(244,235,220,0.8)'
        w = 700 if letter == 'N' else 600
        out.append(f'<text x="{x:.1f}" y="{y:.1f}" fill="{col}" font-size="{size}" font-weight="{w}" '
                   f'text-anchor="middle" dominant-baseline="central" font-family="Inter, sans-serif">{letter}</text>')
    return '\n'.join(out)


def svg_icon(d, size=16, color='#E9D3AE', width=2):
    return (f'<svg width="{size}" height="{size}" viewBox="0 0 24 24" fill="none" stroke="{color}" '
            f'stroke-width="{width}" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">'
            f'<path d="{d}"></path></svg>')


ICONS = dict(
    portal='M12 3a9 9 0 1 0 9 9M12 7a5 5 0 1 0 5 5M12 11a1 1 0 1 0 1 1',
    lifestone='M12 2l5 7-5 13-5-13z',
    house='M3 11l9-7 9 7M5 10v10h14V10M10 20v-6h4v6',
    mansion='M3 21V9l3 2V7l3 2V5l3-2 3 2v4l3-2v4l3-2v12zM10 21v-5h4v5',
    flag='M5 21V4M5 4h11l-2 4 2 4H5',
    market='M4 9l2-5h12l2 5M4 9v11h16V9M4 9h16M9 20v-6h6v6',
    swords='M4 20 15 9M13 5h6v6M20 20 9 9M11 5H5v6',
    spell='M4 20 16 8M14 4l1.5 3L19 8.5 15.5 10 14 13l-1.5-3L9 8.5 12.5 7z',
    plugin='M9 3h6v3a2 2 0 0 0 4 0h2v6h-2a2 2 0 0 0 0 4h2v5h-6v-2a2 2 0 0 0-4 0v2H5v-5H3a2 2 0 0 1 0-4h2V3z',
    move='M12 3v18M3 12h18M9 6l3-3 3 3M9 18l3 3 3-3M6 9l-3 3 3 3M18 9l3 3-3 3',
    close='M6 6l12 12M18 6 6 18',
    check='M5 12l5 5 9-10',
    cross='M6 6l12 12M18 6 6 18',
    search='M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14zM21 21l-4.3-4.3',
    down='M12 4v12M7 11l5 5 5-5M5 20h14',
    chevron='M9 6l6 6-6 6',
    turn='M4 12a8 8 0 0 1 14-5.3M20 4v4h-4M20 12a8 8 0 0 1-14 5.3M4 20v-4h4',
    eye_off='M3 3l18 18M10.6 6.1A9 9 0 0 1 21 12a14 14 0 0 1-2.4 3M6.6 6.6A14 14 0 0 0 3 12s3.5 6 9 6a8 8 0 0 0 3.4-.8',
    reset='M3 12a9 9 0 1 0 3-6.7M3 4v5h5',
    lock='M6 11h12v10H6zM8 11V7a4 4 0 0 1 8 0v4',
    plus='M12 5v14M5 12h14',
    minus='M5 12h14',
    shield='M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6z',
    sword='M14.5 4H20v5.5L9 20.5 3.5 15zM6 13l5 5',
    gem='M6 3h12l3 6-9 12L3 9z',
    coin='M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM9 9h4.5a2 2 0 0 1 0 4H10a2 2 0 0 0 0 4h5M12 6v12',
    sliders='M4 6h10M18 6h2M4 12h4M12 12h8M4 18h12M20 18h0M14 4v4M8 10v4M16 16v4',
    keyboard='M3 6h18v12H3zM7 10h.01M11 10h.01M15 10h.01M7 14h10',
)


def chip(text, fg, bg, edge):
    return (f'<span style="padding: 2px 7px; border-radius: 999px; border: 1px solid {edge}; background: {bg}; '
            f'font-size: 9.5px; font-weight: 700; letter-spacing: 0.06em; color: {fg};">{text}</span>')


OPENAC = chip('OPENAC', ACCENT, 'transparent', 'rgba(201,160,98,0.5)')


def page(title, w, h, body, extra_style='', props=None):
    import json
    p = {'$preview': {'width': w, 'height': h}}
    if props:
        p.update(props)
    return f'''<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>{title}</title>
<script src="./support.js"></script>
</head>
<body>
<x-dc>
<helmet>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&amp;display=swap" rel="stylesheet">
<style>
body{{margin:0}}
button{{font:inherit;color:inherit}}
button:disabled{{cursor:default}}
{extra_style}
</style>
</helmet>
{body}
</x-dc>
<script type="text/x-dc" data-dc-script data-props='{json.dumps(p)}'>
class Component extends DCLogic {{
  renderVals() {{ return {{}}; }}
}}
</script>
</body>
</html>
'''


def write(name, html):
    with open(os.path.join(OUT, name), 'w') as f:
        f.write(html)


def status_line(extra=''):
    return f'''<div style="display: flex; align-items: center; gap: 8px; font-size: 11px; font-weight: 500; font-variant-numeric: tabular-nums; text-shadow: 0 1px 3px rgba(0,0,0,0.9);">
      <span>Holtburg</span><span style="opacity: 0.5;">|</span><span>6:01 AM</span><span style="opacity: 0.5;">|</span><span>38 ms</span><span style="width: 7px; height: 7px; border-radius: 50%; background: #6BD47A;"></span>{extra}
    </div>'''


def world(w, h, inner):
    return f'''<div style="width: {w}px; height: {h}px; position: relative; overflow: hidden; font-family: Inter, system-ui, sans-serif; color: {TEXT}; background: {WORLD};">
  <div style="position: absolute; left: 0; right: 0; top: 46%; height: 34%; background: linear-gradient(180deg, rgba(40,62,36,0) 0%, rgba(40,62,36,0.55) 60%, rgba(40,62,36,0) 100%);"></div>
{inner}
</div>'''


def caption(x, y, w, text):
    return (f'<p style="position: absolute; left: {x}px; top: {y}px; width: {w}px; margin: 0; padding: 10px 12px; '
            f'border-radius: 6px; background: rgba(0,0,0,0.42); font-size: 11.5px; line-height: 1.5; '
            f'color: rgba(255,255,255,0.92); text-shadow: 0 1px 2px rgba(0,0,0,0.7);">{text}</p>')
