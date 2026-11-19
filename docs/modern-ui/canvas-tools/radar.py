import math
from common import *

W, H = 520, 440


def recall_button(left, top):
    return (f'<button aria-label="Recall" title="Recall: lifestone, house, mansion, hometown, market" '
            f'style="position: absolute; left: {left}px; top: {top}px; width: 26px; height: 26px; padding: 0; '
            f'border-radius: 50%; border: 1px solid {TRIM}; background: rgba(0,0,0,0.62); display: flex; '
            f'align-items: center; justify-content: center; cursor: pointer;">{svg_icon(ICONS["portal"], 14, ACCENT, 2.2)}</button>')


# ---------- A: now ----------
def board_now():
    r = 80
    svg = f'''<svg width="160" height="160" viewBox="0 0 160 160" style="position: absolute; inset: 0;" aria-hidden="true">
        {blips_svg(r, dots=True, sel_ring=False)}
        {player_arrow(r, 6)}
      </svg>'''
    inner = f'''
  <div style="position: absolute; right: 24px; top: 20px; display: flex; flex-direction: column; align-items: center; gap: 8px;">
    <div style="align-self: flex-end;">{status_line()}</div>
    <div role="img" aria-label="Radar" style="width: 160px; height: 160px; position: relative; border-radius: 50%; border: 2px solid {TRIM}; background: rgba(0,0,0,0.38); box-shadow: 0 2px 10px rgba(0,0,0,0.35);">
      <div style="position: absolute; inset: 26px; border-radius: 50%; border: 1px solid rgba(255,255,255,0.12);"></div>
      {svg}
      <span style="position: absolute; left: 50%; top: 3px; transform: translateX(-50%); font-size: 11px; font-weight: 700; color: {ACCENT};">N</span>
      <span style="position: absolute; right: 5px; top: 50%; transform: translateY(-50%); font-size: 10px; font-weight: 600; opacity: 0.8;">E</span>
      <span style="position: absolute; left: 50%; bottom: 3px; transform: translateX(-50%); font-size: 10px; font-weight: 600; opacity: 0.8;">S</span>
      <span style="position: absolute; left: 5px; top: 50%; transform: translateY(-50%); font-size: 10px; font-weight: 600; opacity: 0.8;">W</span>
      <div style="position: absolute; right: -6px; bottom: 4px; display: flex; flex-direction: column; gap: 4px;">
        <button aria-label="Zoom radar in" style="width: 22px; height: 22px; border-radius: 50%; border: 1px solid {TRIM}; background: rgba(0,0,0,0.6); font-size: 13px; line-height: 1; padding: 0; cursor: pointer;">+</button>
        <button aria-label="Zoom radar out" style="width: 22px; height: 22px; border-radius: 50%; border: 1px solid {TRIM}; background: rgba(0,0,0,0.6); font-size: 13px; line-height: 1; padding: 0; cursor: pointer;">−</button>
      </div>
    </div>
    <span style="font-size: 11px; font-weight: 600; font-variant-numeric: tabular-nums; text-shadow: 0 1px 3px rgba(0,0,0,0.9);">42.1N 33.6E</span>
  </div>
  {caption(24, 300, 300, "As designed so far: a dark disc with a bronze rim, north always up, round dots in three colours, + and − to zoom, coordinates under it. The dots lose retail's shapes (fellow, leader, allegiance, PK), and the colours were not retail's.")}'''
    write('RadarNow.dc.html', page('Radar: as designed', W, H, world(W, H, inner)))


# ---------- B: bearing ring ----------
def board_bearing():
    R = 92            # outer
    band = 15
    ri = R - band     # inner disc radius
    ticks = []
    for deg in range(0, 360, 15):
        a = math.radians(deg - HEADING)
        major = deg % 90 == 0
        if major:
            continue
        l1 = R - 3
        l2 = R - (7 if deg % 45 == 0 else 5)
        x1, y1 = R + math.sin(a) * l1, R - math.cos(a) * l1
        x2, y2 = R + math.sin(a) * l2, R - math.cos(a) * l2
        ticks.append(f'<line x1="{x1:.1f}" y1="{y1:.1f}" x2="{x2:.1f}" y2="{y2:.1f}" stroke="rgba(244,235,220,{0.75 if deg % 45 == 0 else 0.4})" stroke-width="1.2"></line>')
    rings = ''.join(
        f'<circle cx="{R}" cy="{R}" r="{ri*f:.1f}" fill="none" stroke="rgba(255,255,255,0.13)" stroke-dasharray="2 3"></circle>'
        for f in (1/3, 2/3))
    labels = (f'<text x="{R+3}" y="{R - ri/3 - 3:.1f}" fill="rgba(244,235,220,0.45)" font-size="8" font-family="Inter, sans-serif">25</text>'
              f'<text x="{R+3}" y="{R - 2*ri/3 - 3:.1f}" fill="rgba(244,235,220,0.45)" font-size="8" font-family="Inter, sans-serif">50</text>')
    sel = BLIPS[0]
    sx, sy = R + sel['x'] * ri, R + sel['y'] * ri
    svg = f'''<svg width="{2*R}" height="{2*R}" viewBox="0 0 {2*R} {2*R}" style="position: absolute; inset: 0;" aria-hidden="true">
        <circle cx="{R}" cy="{R}" r="{R-1}" fill="rgba(0,0,0,0.5)" stroke="{TRIM}" stroke-width="1.5"></circle>
        <circle cx="{R}" cy="{R}" r="{ri}" fill="rgba(0,0,0,0.22)" stroke="rgba(201,160,98,0.45)" stroke-width="1"></circle>
        {''.join(ticks)}
        {compass_letters(R, band/2 + 0.5, 10)}
        {rings}{labels}
        <path d="M{R} {R}L{R - ri*math.sin(math.radians(30)):.1f} {R - ri*math.cos(math.radians(30)):.1f}A{ri} {ri} 0 0 1 {R + ri*math.sin(math.radians(30)):.1f} {R - ri*math.cos(math.radians(30)):.1f}z" fill="rgba(255,255,255,0.05)"></path>
        {blips_svg(ri, s=4, c=R)}
        {player_arrow(R, 7)}
      </svg>'''
    inner = f'''
  <div style="position: absolute; right: 24px; top: 20px; display: flex; flex-direction: column; align-items: center; gap: 6px;">
    <div style="align-self: flex-end;">{status_line()}</div>
    <div style="display: flex; align-items: center; gap: 6px; height: 20px; padding: 0 10px; border-radius: 999px; border: 1px solid {TRIM}; background: rgba(0,0,0,0.55); font-size: 11px; font-weight: 700; font-variant-numeric: tabular-nums;">
      <span style="color: {ACCENT};">NE</span><span>42°</span>
    </div>
    <div style="width: 0; height: 0; margin-bottom: -6px; border-left: 5px solid transparent; border-right: 5px solid transparent; border-top: 6px solid {ACCENT};"></div>
    <div role="img" aria-label="Radar, heading up" style="width: {2*R}px; height: {2*R}px; position: relative;">
      {svg}
      {recall_button(-8, 2*R - 30)}
      <button title="Range: 75 m. Click for 40 m, 20 m." style="position: absolute; right: -10px; bottom: 2px; height: 22px; padding: 0 8px; border-radius: 999px; border: 1px solid {TRIM}; background: rgba(0,0,0,0.62); font-size: 10.5px; font-weight: 600; font-variant-numeric: tabular-nums; cursor: pointer;">75 m</button>
      <div style="position: absolute; left: {sx + 12:.0f}px; top: {sy - 30:.0f}px; padding: 4px 8px; border-radius: 4px; border: 1px solid {TRIM}; background: rgba(20,16,12,0.94); font-size: 11px; white-space: nowrap; box-shadow: 0 6px 16px rgba(0,0,0,0.5);">
        <strong style="font-weight: 600;">Drudge Skulker</strong> <span style="color: {SUB};">· 31 m · selected</span>
      </div>
    </div>
    <span style="font-size: 11px; font-weight: 600; font-variant-numeric: tabular-nums; text-shadow: 0 1px 3px rgba(0,0,0,0.9);">42.1N 33.6E</span>
  </div>
  {caption(24, 250, 250, "Heading up, as retail: you always face the top and a compass ring with ticks turns around you, N in bronze. The heading sits above it (NE 42°). Retail's shapes and colours come back: gold creatures, green fellows (▲ leader, ▼ member), □ allegiance, purple portals, yellow vendors. Faint rings at 25 and 50 m and a soft cone for your view. Hovering a blip names it. Range chip and Recall on the rim.")}'''
    write('RadarBearing.dc.html', page('Radar: bearing ring', W, H, world(W, H, inner)))


# ---------- C: lens ----------
def board_lens():
    R = 84
    a = math.radians(-HEADING)
    nx, ny = R + math.sin(a) * (R - 4), R - math.cos(a) * (R - 4)
    tx, ty = R + math.sin(a) * (R - 16), R - math.cos(a) * (R - 16)
    svg = f'''<svg width="{2*R}" height="{2*R}" viewBox="0 0 {2*R} {2*R}" style="position: absolute; inset: 0; overflow: visible;" aria-hidden="true">
        <defs><filter id="glow" x="-1" y="-1" width="3" height="3"><feGaussianBlur stdDeviation="1.6" result="b"></feGaussianBlur><feMerge><feMergeNode in="b"></feMergeNode><feMergeNode in="SourceGraphic"></feMergeNode></feMerge></filter></defs>
        <circle cx="{nx:.1f}" cy="{ny:.1f}" r="3" fill="{ACCENT}"></circle>
        <text x="{tx:.1f}" y="{ty:.1f}" fill="{ACCENT}" font-size="10" font-weight="700" text-anchor="middle" dominant-baseline="central" font-family="Inter, sans-serif">N</text>
        {blips_svg(R - 6, s=4, glow=True, c=R)}
        {player_arrow(R, 6)}
      </svg>'''
    inner = f'''
  <div style="position: absolute; right: 24px; top: 20px; display: flex; flex-direction: column; align-items: center; gap: 6px;">
    <div style="align-self: flex-end;">{status_line()}</div>
    <div role="img" aria-label="Radar, heading up" style="width: {2*R}px; height: {2*R}px; position: relative; border-radius: 50%; background: radial-gradient(circle, rgba(0,0,0,0.42) 0%, rgba(0,0,0,0.32) 58%, rgba(0,0,0,0) 72%);">
      {svg}
    </div>
    <span style="padding: 3px 9px; border-radius: 999px; background: rgba(0,0,0,0.45); font-size: 10.5px; font-weight: 600; font-variant-numeric: tabular-nums; color: rgba(244,235,220,0.9);">42.1N 33.6E · 75 m</span>
  </div>
  <div style="position: absolute; right: 230px; top: 132px; padding: 4px 8px; border-radius: 4px; background: rgba(0,0,0,0.5); font-size: 10px; color: {SUB};">hovering: coordinates and range show</div>
  {caption(24, 270, 300, "Lens: no disc, no rim. A soft dark pool fades into the world and the blips glow on it; only a bronze N dot on the edge turns as you do. Coordinates and range appear while the pointer is over it, and Recall slides out on hover too. The lightest option: the world shows through almost everywhere.")}'''
    write('RadarLens.dc.html', page('Radar: lens', W, H, world(W, H, inner)))


# ---------- D: terrain ----------
def land_svg(size, rot):
    c = size / 2
    return f'''<g transform="rotate({rot} {c} {c})">
          <rect x="-80" y="-80" width="{size+160}" height="{size+160}" fill="#5D7A46"></rect>
          <path d="M-80 {c+30}C20 {c+10} 60 {c-60} {size+80} {c-40}" stroke="#4E7FA6" stroke-width="9" fill="none" opacity="0.9"></path>
          <path d="M{c-10} -80C{c-6} 40 {c+18} 120 {c+6} {size+80}" stroke="#CDB98C" stroke-width="5" fill="none" opacity="0.85"></path>
          <path d="M-80 {c-20}H{size+80}" stroke="#CDB98C" stroke-width="3" fill="none" opacity="0.6"></path>
          <ellipse cx="{c-60}" cy="{c+60}" rx="34" ry="22" fill="#45612F"></ellipse>
          <ellipse cx="{c+70}" cy="{c-70}" rx="40" ry="26" fill="#45612F"></ellipse>
          <ellipse cx="{c+60}" cy="{c+70}" rx="22" ry="16" fill="#4A6733"></ellipse>
          <rect x="{c+14}" y="{c-30}" width="12" height="10" fill="#8C7A60"></rect>
          <rect x="{c+30}" y="{c-14}" width="10" height="12" fill="#8C7A60"></rect>
          <rect x="{c-34}" y="{c-46}" width="14" height="10" fill="#8C7A60"></rect>
        </g>'''


def board_terrain():
    S = 204
    c = S / 2
    a = math.radians(-HEADING)
    nx, ny = c + math.sin(a) * (c - 12), c - math.cos(a) * (c - 12)
    svg = f'''<svg width="{S}" height="{S}" viewBox="0 0 {S} {S}" style="position: absolute; inset: 0;" aria-hidden="true">
        {land_svg(S, -HEADING)}
        <rect width="{S}" height="{S}" fill="rgba(0,0,0,0.12)"></rect>
        {blips_svg(c - 14, s=4, c=c)}
        {player_arrow(c, 7)}
        <circle cx="{nx:.1f}" cy="{ny:.1f}" r="8" fill="rgba(20,16,12,0.85)" stroke="{ACCENT}" stroke-width="1"></circle>
        <text x="{nx:.1f}" y="{ny:.1f}" fill="{ACCENT}" font-size="9.5" font-weight="700" text-anchor="middle" dominant-baseline="central" font-family="Inter, sans-serif">N</text>
      </svg>'''
    indoor = f'''<svg width="120" height="120" viewBox="0 0 120 120" style="position: absolute; inset: 0;" aria-hidden="true">
        <rect width="120" height="120" fill="#17130F"></rect>
        <path d="M20 0V50H0M44 0V36H76V0M100 0V50H120M0 76H44V120M76 120V76H120M44 76V64M76 76V64" stroke="#8C7A60" stroke-width="3" fill="none"></path>
        <rect x="44" y="50" width="32" height="14" fill="#2A231B"></rect>
        <path d="M95 92h8M99 88v8" stroke="{GOLD}" stroke-width="2" stroke-linecap="round"></path>
        <path d="M60 50l5 10-5-3-5 3z" fill="#FFFFFF"></path>
      </svg>'''
    inner = f'''
  <div style="position: absolute; right: 24px; top: 20px; display: flex; flex-direction: column; align-items: flex-end; gap: 8px;">
    {status_line()}
    <div role="img" aria-label="Radar over the land, heading up" style="width: {S}px; height: {S}px; position: relative; border-radius: 14px; overflow: hidden; border: 1.5px solid {TRIM}; box-shadow: 0 4px 14px rgba(0,0,0,0.4);">
      {svg}
      <span style="position: absolute; right: 8px; top: 7px; padding: 1px 6px; border-radius: 999px; background: rgba(0,0,0,0.55); font-size: 10px; font-weight: 600;">75 m</span>
      <div style="position: absolute; left: 0; right: 0; bottom: 0; height: 26px; padding: 0 10px; display: flex; align-items: center; justify-content: space-between; background: linear-gradient(180deg, rgba(0,0,0,0), rgba(0,0,0,0.7)); font-size: 10.5px; font-weight: 600; font-variant-numeric: tabular-nums;">
        <span>Holtburg</span><span>42.1N 33.6E</span>
      </div>
    </div>
    <div style="position: relative; width: {S}px; height: 0;">{recall_button(-14, -46)}</div>
  </div>
  <div style="position: absolute; left: 24px; top: 20px; display: flex; flex-direction: column; gap: 6px;">
    <div role="img" aria-label="Indoors" style="width: 120px; height: 120px; position: relative; border-radius: 10px; overflow: hidden; border: 1.5px solid {TRIM};">{indoor}</div>
    <span style="font-size: 10.5px; font-weight: 600; text-shadow: 0 1px 2px rgba(0,0,0,0.8);">Indoors: walls, 25 m</span>
  </div>
  {caption(24, 250, 250, "Terrain: a rounded square showing the land itself (roads, water, trees, buildings, from the game's own data) turning heading up, blips on top. Place and coordinates sit in its foot. Indoors it draws the walls around you at retail's 25 m range. The most to read and the most to build.")}'''
    write('RadarTerrain.dc.html', page('Radar: terrain', W, H, world(W, H, inner)))


# ---------- E: compass strip ----------
def board_strip():
    W2, H2 = 1040, 340
    SW = 640
    left = (W2 - SW) / 2
    ticks, marks = [], []
    for deg in range(-90, 91, 15):
        x = SW / 2 + deg / 90 * (SW / 2)
        bearing = (HEADING + deg) % 360
        names = {0: 'N', 45: 'NE', 90: 'E', 135: 'SE', 180: 'S', 225: 'SW', 270: 'W', 315: 'NW'}
        if bearing in names:
            col = ACCENT if names[bearing] == 'N' else TEXT
            ticks.append(f'<span style="position: absolute; left: {x - 12:.0f}px; top: 3px; width: 24px; text-align: center; font-size: 11px; font-weight: 700; color: {col};">{names[bearing]}</span>')
        else:
            ticks.append(f'<span style="position: absolute; left: {x:.0f}px; top: 6px; width: 1px; height: 7px; background: rgba(244,235,220,0.5);"></span>')
    behind_l = behind_r = 0
    for b in BLIPS:
        rel = math.degrees(math.atan2(b['x'], -b['y']))
        if abs(rel) > 90:
            if rel < 0:
                behind_l += 1
            else:
                behind_r += 1
            continue
        x = SW / 2 + rel / 90 * (SW / 2)
        s = 4
        svgb = blip_shape(b['shape'], 7, 7, s, b['color'])
        ring = f'<circle cx="7" cy="7" r="7" fill="none" stroke="{ACCENT}" stroke-width="1.3"></circle>' if b.get('sel') else ''
        marks.append(f'<svg width="14" height="14" viewBox="0 0 14 14" style="position: absolute; left: {x - 7:.0f}px; top: 17px; overflow: visible;" aria-hidden="true">{svgb}{ring}</svg>')
        if b.get('sel'):
            marks.append(f'<span style="position: absolute; left: {x - 20:.0f}px; top: 34px; width: 40px; text-align: center; font-size: 10px; font-weight: 600; color: {ACCENT}; font-variant-numeric: tabular-nums;">31 m</span>')

    def bar(color, pct, label):
        return f'''<div style="flex: 1; display: flex; flex-direction: column; gap: 2px;">
          <div style="height: 10px; border-radius: 3px; background: rgba(0,0,0,0.5); overflow: hidden;"><div style="width: {pct}%; height: 100%; background: {color};"></div></div>
          <span style="font-size: 10px; font-weight: 600; text-align: center; font-variant-numeric: tabular-nums; text-shadow: 0 1px 2px rgba(0,0,0,0.9);">{label}</span>
        </div>'''
    inner = f'''
  <div role="img" aria-label="Compass strip" style="position: absolute; left: {left:.0f}px; top: 10px; width: {SW}px; height: 32px; border-radius: 6px; border: 1px solid {TRIM_SOFT}; background: linear-gradient(90deg, rgba(0,0,0,0), rgba(0,0,0,0.5) 12%, rgba(0,0,0,0.5) 88%, rgba(0,0,0,0));">
    {''.join(ticks)}
    {''.join(marks)}
    <span style="position: absolute; left: {SW/2 - 5:.0f}px; top: -1px; width: 0; height: 0; border-left: 5px solid transparent; border-right: 5px solid transparent; border-top: 6px solid {ACCENT};"></span>
    <span style="position: absolute; left: -64px; top: 8px; font-size: 10px; color: {SUB}; white-space: nowrap;">◂ {behind_l} behind</span>
    <span style="position: absolute; right: -64px; top: 8px; font-size: 10px; color: {SUB}; white-space: nowrap;">{behind_r} behind ▸</span>
  </div>
  <div style="position: absolute; left: {(W2 - 600)/2:.0f}px; top: 56px; width: 600px; display: flex; gap: 8px;">
    {bar('#E5484D', 82, '412 / 502')}{bar('#E3B341', 64, '388 / 606')}{bar('#4C8DFF', 91, '512 / 560')}
  </div>
  <div style="position: absolute; left: {(W2 - 300)/2:.0f}px; top: 96px; width: 300px; text-align: center; font-size: 12px; font-weight: 600; text-shadow: 0 1px 3px rgba(0,0,0,0.85);">Drudge Skulker <span style="color: {SUB}; font-weight: 500;">· level 12</span></div>
  <div style="position: absolute; right: 16px; top: 14px; display: flex; flex-direction: column; align-items: flex-end; gap: 6px;">
    {status_line()}
    <span style="font-size: 11px; font-weight: 600; font-variant-numeric: tabular-nums; text-shadow: 0 1px 3px rgba(0,0,0,0.9);">42.1N 33.6E · facing NE 42°</span>
  </div>
  {caption(24, 180, 470, "Compass strip: no round radar at all. A strip across the top shows what is in front of you (left to right as you look), with retail's blip shapes at their bearing and the selected one's distance under it. What is behind you is only counted at the ends. Frees the top right corner, but you lose the picture of what is around you, so the round radar stays an option.")}'''
    write('RadarStrip.dc.html', page('Radar: compass strip', W2, H2, world(W2, H2, inner)))


board_now()
board_bearing()
board_lens()
board_terrain()
board_strip()
print('radar boards written')
