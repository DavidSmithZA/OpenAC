from common import *

KEEP = ('KEEP', '#7FD08A', 'rgba(127,208,138,0.14)', 'rgba(127,208,138,0.6)')
SALV = ('SALVAGE', '#E3B341', 'rgba(227,179,65,0.14)', 'rgba(227,179,65,0.6)')
SELL = ('SELL', '#7CB7FF', 'rgba(124,183,255,0.14)', 'rgba(124,183,255,0.6)')


def btn(label, primary=False, extra=''):
    if primary:
        return (f'<button style="height: 32px; padding: 0 14px; border-radius: 4px; border: 1px solid {ACCENT}; '
                f'background: rgba(224,176,104,0.9); color: #1A140D; font-size: 12px; font-weight: 700; cursor: pointer; {extra}">{label}</button>')
    return (f'<button style="height: 32px; padding: 0 12px; border-radius: 4px; border: 1px solid rgba(201,160,98,0.6); '
            f'background: rgba(0,0,0,0.35); font-size: 12px; font-weight: 600; cursor: pointer; display: flex; align-items: center; gap: 6px; {extra}">{label}</button>')


def window(w, h, inner, extra=''):
    return (f'<div style="width: {w}px; height: {h}px; position: relative; box-sizing: border-box; display: flex; flex-direction: column; '
            f'font-family: Inter, system-ui, sans-serif; color: {TEXT}; background: {PANEL}; border: 1px solid {TRIM}; '
            f'border-radius: 6px; box-shadow: 0 18px 50px rgba(0,0,0,0.5); overflow: hidden; {extra}">{inner}</div>')


def header(icon, title, sub, tint=ACCENT):
    return f'''<div style="flex: none; height: 56px; display: flex; align-items: center; gap: 10px; padding: 0 10px 0 14px; border-bottom: 1px solid {TRIM_SOFT}; background: linear-gradient(180deg, rgba(201,160,98,0.10), rgba(201,160,98,0));">
    <span style="width: 34px; height: 34px; flex: none; border-radius: 4px; border: 1px solid rgba(201,160,98,0.6); background: linear-gradient(160deg, rgba(224,176,104,0.2), rgba(0,0,0,0.5)); display: flex; align-items: center; justify-content: center;">{svg_icon(ICONS[icon], 18, tint)}</span>
    <span style="flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px;">
      <h1 style="margin: 0; font-size: 14px; font-weight: 700; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">{title}</h1>
      <span style="font-size: 11px; color: {SUB}; font-variant-numeric: tabular-nums;">{sub}</span>
    </span>
    <button aria-label="Close" style="width: 30px; height: 30px; padding: 0; border-radius: 4px; border: 0; background: transparent; display: flex; align-items: center; justify-content: center; cursor: pointer;">{svg_icon(ICONS['close'], 16)}</button>
  </div>'''


def verdict(v):
    return chip(v[0], v[1], v[2], v[3])


def section_label(text, icon=None):
    ic = svg_icon(ICONS[icon], 11, SUB, 2.2) if icon else ''
    return (f'<span style="display: flex; align-items: center; gap: 6px; padding: 4px 10px 2px; font-size: 9.5px; '
            f'font-weight: 700; letter-spacing: 0.12em; color: {SUB};">{ic}{text}</span>')


# ---------------- recalls ----------------
def recalls():
    W, H = 1000, 800
    places = [
        ('lifestone', 'Lifestone', 'The last lifestone you tied to', '@ls', None),
        ('house', 'House', 'Cottage, 12.4N 40.1E', '@hr', None),
        ('mansion', 'Allegiance mansion', "Your monarch's mansion", '@hom', None),
        ('flag', 'Allegiance hometown', 'Set by your monarch', '@ah', None),
        ('market', 'Marketplace', 'The Marketplace', '@mp', None),
        ('swords', 'Player killer arena', 'Player killers only', '@pkarena', 'off'),
        ('swords', 'PK lite arena', 'PK lite only', '@pklarena', 'off'),
    ]
    spells = [
        ('Portal Recall', 'The last portal you went through'),
        ('Lifestone Recall', 'As @ls, cast as a spell'),
        ('Primary Portal Recall', 'Your primary portal tie'),
        ('Secondary Portal Recall', 'Your secondary portal tie'),
    ]
    rows = []
    for i, (ic, name, sub, cmd, off) in enumerate(places):
        hover = 'background: rgba(224,176,104,0.14);' if i == 0 else 'background: transparent;'
        dis = ' disabled' if off else ''
        op = 'opacity: 0.45;' if off else ''
        rows.append(f'''<button role="menuitem"{dis} title="{sub}" style="height: 40px; padding: 0 10px; border: 0; border-radius: 4px; {hover} {op} display: flex; align-items: center; gap: 10px; cursor: pointer; text-align: left;">
        {svg_icon(ICONS[ic], 15, ACCENT)}
        <span style="flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 1px;"><span style="font-size: 12.5px; font-weight: 600;">{name}</span><span style="font-size: 10.5px; color: {MUTED};">{sub}</span></span>
        <span style="font-size: 10.5px; color: {MUTED}; font-variant-numeric: tabular-nums;">{cmd}</span>
      </button>''')
    srows = []
    for name, sub in spells:
        srows.append(f'''<button role="menuitem" disabled title="Switch to Magic mode to cast" style="height: 40px; padding: 0 10px; border: 0; border-radius: 4px; background: transparent; opacity: 0.55; display: flex; align-items: center; gap: 10px; text-align: left;">
        {svg_icon(ICONS['spell'], 15, '#C9A2FF')}
        <span style="flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 1px;"><span style="font-size: 12.5px; font-weight: 600;">{name}</span><span style="font-size: 10.5px; color: {MUTED};">{sub}</span></span>
        <span style="font-size: 10px; color: {MUTED};">Magic mode</span>
      </button>''')
    menu = f'''<div role="menu" aria-label="Recall" style="position: absolute; left: 476px; top: 40px; width: 316px; box-sizing: border-box; padding: 4px; border-radius: 6px; border: 1px solid {TRIM}; background: rgba(24,19,14,0.97); box-shadow: 0 14px 34px rgba(0,0,0,0.6); display: flex; flex-direction: column;">
      <div style="height: 34px; padding: 0 10px; display: flex; align-items: center; gap: 8px; border-bottom: 1px solid {TRIM_SOFT}; margin-bottom: 4px;">
        {svg_icon(ICONS['portal'], 15, ACCENT, 2.2)}<span style="flex: 1; font-size: 13px; font-weight: 700;">Recall</span>
        <span style="font-size: 10.5px; color: {MUTED};">Key: none set</span>
      </div>
      {section_label('PLACES')}
      {''.join(rows)}
      <span style="height: 1px; margin: 4px 6px; background: rgba(201,160,98,0.25);"></span>
      {section_label('YOUR RECALL SPELLS')}
      {''.join(srows)}
      <span style="height: 1px; margin: 4px 6px; background: rgba(201,160,98,0.25);"></span>
      {section_label('YOUR OWN · DREAMWEAVE')}
      <button role="menuitem" title="Sends /dn as if you typed it. Right-click to edit, move or remove." style="height: 40px; padding: 0 10px; border: 0; border-radius: 4px; background: transparent; display: flex; align-items: center; gap: 10px; cursor: pointer; text-align: left;">
        {svg_icon(ICONS['portal'], 15, ACCENT)}
        <span style="flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 1px;"><span style="font-size: 12.5px; font-weight: 600;">Driftnet</span><span style="font-size: 10.5px; color: {MUTED};">This server only</span></span>
        <span style="font-size: 10.5px; color: {MUTED};">/dn</span>
      </button>
      <button role="menuitem" aria-expanded="true" style="height: 34px; padding: 0 10px; border: 0; border-radius: 4px; background: rgba(224,176,104,0.14); display: flex; align-items: center; gap: 10px; font-size: 12.5px; font-weight: 600; color: {ACCENT}; cursor: pointer; text-align: left;">{svg_icon(ICONS['plus'], 14, ACCENT, 2.2)}Add your own…</button>
    </div>'''
    def field(label, value, note=''):
        n = f'<span style="font-size: 10.5px; color: {MUTED};">{note}</span>' if note else ''
        return f'''<label style="display: flex; flex-direction: column; gap: 4px;"><span style="font-size: 11px; font-weight: 600; color: {SUB};">{label}</span>
          <input value="{value}" style="height: 30px; box-sizing: border-box; padding: 0 10px; border-radius: 4px; border: 1px solid rgba(201,160,98,0.6); background: rgba(0,0,0,0.4); color: {TEXT}; font: inherit; font-size: 12.5px;">{n}</label>'''
    icons = ''.join(f'<button aria-label="{n}" style="width: 30px; height: 30px; padding: 0; border-radius: 4px; border: 1px solid {ACCENT if n == "portal" else "rgba(201,160,98,0.35)"}; background: {"rgba(224,176,104,0.16)" if n == "portal" else "rgba(0,0,0,0.3)"}; display: flex; align-items: center; justify-content: center; cursor: pointer;">{svg_icon(ICONS[n], 14, ACCENT)}</button>' for n in ('portal', 'flag', 'house', 'market', 'lifestone', 'swords'))
    seg = ''.join(f'<button aria-pressed="{"true" if i == 0 else "false"}" style="flex: 1; height: 28px; border: 0; border-radius: 3px; background: {"rgba(224,176,104,0.9)" if i == 0 else "transparent"}; color: {"#1A140D" if i == 0 else TEXT}; font-size: 11.5px; font-weight: 700; cursor: pointer;">{t}</button>' for i, t in enumerate(('Dreamweave only', 'Every server')))
    form = f'''<div role="dialog" aria-label="Add a recall" style="position: absolute; left: 24px; top: 330px; width: 380px; box-sizing: border-box; border-radius: 6px; border: 1px solid {TRIM}; background: rgba(20,16,12,0.96); box-shadow: 0 14px 34px rgba(0,0,0,0.55); font-size: 12px;">
    <div style="height: 40px; padding: 0 14px; display: flex; align-items: center; gap: 8px; border-bottom: 1px solid {TRIM_SOFT};">{svg_icon(ICONS['plus'], 14, ACCENT, 2.2)}<h2 style="flex: 1; margin: 0; font-size: 13px; font-weight: 700;">Add a recall</h2></div>
    <div style="padding: 12px 14px; display: flex; flex-direction: column; gap: 12px;">
      {field('Name', 'Driftnet')}
      {field('Command', '/dn', 'Sent as if you typed it in chat. Any / or @ command the server knows.')}
      <div style="display: flex; flex-direction: column; gap: 4px;"><span style="font-size: 11px; font-weight: 600; color: {SUB};">Icon</span><div style="display: flex; gap: 6px;">{icons}</div></div>
      <div style="display: flex; flex-direction: column; gap: 4px;"><span style="font-size: 11px; font-weight: 600; color: {SUB};">Show it on</span><div style="display: flex; gap: 2px; padding: 2px; border-radius: 4px; border: 1px solid rgba(201,160,98,0.4);">{seg}</div></div>
    </div>
    <div style="height: 50px; padding: 0 14px; border-top: 1px solid {TRIM_SOFT}; display: flex; align-items: center; justify-content: flex-end; gap: 8px;">
      <button style="height: 30px; padding: 0 12px; border-radius: 4px; border: 1px solid rgba(201,160,98,0.6); background: rgba(0,0,0,0.35); font-size: 12px; font-weight: 600; cursor: pointer;">Cancel</button>
      <button style="height: 30px; padding: 0 14px; border-radius: 4px; border: 1px solid {ACCENT}; background: rgba(224,176,104,0.9); color: #1A140D; font-size: 12px; font-weight: 700; cursor: pointer;">Add</button>
    </div>
  </div>'''
    radar = f'''<div style="position: absolute; right: 24px; top: 20px; display: flex; flex-direction: column; align-items: center; gap: 8px;">
    <div style="align-self: flex-end;">{status_line()}</div>
    <div style="width: 160px; height: 160px; position: relative; border-radius: 50%; border: 2px solid {TRIM}; background: rgba(0,0,0,0.38);">
      <svg width="160" height="160" viewBox="0 0 160 160" style="position: absolute; inset: 0;" aria-hidden="true">{blips_svg(72, s=4, c=80)}{player_arrow(80, 6)}</svg>
      <button aria-label="Recall" aria-expanded="true" style="position: absolute; left: -8px; bottom: 2px; width: 28px; height: 28px; padding: 0; border-radius: 50%; border: 1px solid {ACCENT}; background: rgba(224,176,104,0.9); display: flex; align-items: center; justify-content: center; cursor: pointer;">{svg_icon(ICONS['portal'], 15, '#1A140D', 2.4)}</button>
    </div>
    <span style="font-size: 11px; font-weight: 600; font-variant-numeric: tabular-nums; text-shadow: 0 1px 3px rgba(0,0,0,0.9);">42.1N 33.6E</span>
  </div>'''
    inner = radar + menu + form + caption(24, 40, 380, "The Recall button sits on the radar's rim. Hover shows what it holds; a click opens the list. Each place sends the same request as typing its @ command (shown on the right, so players learn them). Places you cannot use are greyed with the reason. Your recall spells follow, greyed outside Magic mode like spells on a bar. Drag any row onto a shortcut bar to make it a slot. A recall the server refuses says why under the target. Your own: any command a server adds (Driftnet on Dreamweave is /dn), saved for that server or for all; right-click one to edit, move or remove it.")
    write('Recalls.dc.html', page('Recall menu', W, H, world(W, H, inner)))


# ---------------- quick move ----------------
def quick_move():
    W, H = 1440, 900
    pieces = [
        ('Effects', 10, 8, 212, 40),
        ('Vitals', 359, 8, 722, 30),
        ('Target', 504, 42, 432, 64),
        ('Chat', 10, 694, 432, 198),
        ('Main bar', 442, 686, 556, 206),
        ('Panels', 1108, 836, 322, 56),
    ]
    boxes = []
    for name, x, y, w, h in pieces:
        boxes.append(f'''<div style="position: absolute; left: {x}px; top: {y}px; width: {w}px; height: {h}px; box-sizing: border-box; border: 1.5px dashed rgba(224,176,104,0.85); border-radius: 6px; background: rgba(224,176,104,0.06);">
      <span style="position: absolute; left: 4px; top: {'-20' if y > 30 else str(h + 4)}px; display: flex; align-items: center; gap: 4px; padding: 1px 6px; border-radius: 3px; background: rgba(20,16,12,0.85); font-size: 10px; font-weight: 600; white-space: nowrap;">{svg_icon(ICONS['move'], 10, ACCENT, 2.2)}{name}</span>
    </div>''')
    rx, ry, rw, rh = 1218, 6, 212, 226
    radar = f'''<div style="position: absolute; left: {rx}px; top: {ry}px; width: {rw}px; height: {rh}px; box-sizing: border-box; border: 2px solid {ACCENT}; border-radius: 6px; background: rgba(224,176,104,0.12); box-shadow: 0 0 0 4px rgba(224,176,104,0.18);">
      <span style="position: absolute; left: 4px; top: {rh + 6}px; display: flex; align-items: center; gap: 6px; padding: 2px 8px; border-radius: 3px; background: rgba(20,16,12,0.92); font-size: 10.5px; font-weight: 600; white-space: nowrap; font-variant-numeric: tabular-nums;">{svg_icon(ICONS['move'], 11, ACCENT, 2.2)}Radar · 16 from the right · 12 from the top</span>
      <span style="position: absolute; left: 92px; top: 96px;">{svg_icon(ICONS['move'], 26, '#FFFFFF', 2.4)}</span>
    </div>
    <div style="position: absolute; left: 0; right: 0; top: 12px; height: 0; border-top: 1px solid rgba(124,196,255,0.9);"></div>
    <span style="position: absolute; left: 1090px; top: 14px; padding: 1px 6px; border-radius: 3px; background: rgba(20,40,60,0.85); font-size: 10px; color: #BFE2FF;">lines up with the vitals</span>
    <div style="position: absolute; top: 0; bottom: 0; left: 1424px; width: 0; border-left: 1px solid rgba(124,196,255,0.9);"></div>
    <span style="position: absolute; left: 1338px; top: 250px; padding: 1px 6px; border-radius: 3px; background: rgba(20,40,60,0.85); font-size: 10px; color: #BFE2FF;">16 from the edge</span>'''
    pill = f'''<div style="position: absolute; left: 50%; top: 124px; transform: translateX(-50%); display: flex; align-items: center; gap: 10px; height: 34px; padding: 0 16px; border-radius: 999px; border: 1px solid {TRIM}; background: rgba(20,16,12,0.9); font-size: 12px; white-space: nowrap; box-shadow: 0 6px 18px rgba(0,0,0,0.45);">
    <span style="padding: 1px 7px; border-radius: 4px; border: 1px solid rgba(244,235,220,0.5); font-size: 11px; font-weight: 700;">Ctrl</span>
    <span>held: drag anything to move it</span><span style="opacity: 0.5;">·</span><span>Ctrl + right-click for more</span><span style="opacity: 0.5;">·</span><span style="color: {SUB};">let go of Ctrl to finish</span>
  </div>'''
    body = f'''<div style="width: {W}px; height: {H}px; position: relative; overflow: hidden; font-family: Inter, system-ui, sans-serif; color: {TEXT};">
  <dc-import name="Horizon" trim="bronze" hint-size="1440px,900px"></dc-import>
  <div style="position: absolute; inset: 0; background: rgba(0,0,0,0.12);"></div>
  {''.join(boxes)}
  {radar}
  {pill}
</div>'''
    write('QuickMove.dc.html', page('Quick move with Ctrl', W, H, body))


def quick_menu():
    W, H = 900, 460
    slots = ''.join(
        f'<span style="width: 40px; height: 40px; border-radius: 4px; border: 1px solid rgba(201,160,98,0.45); background: rgba(0,0,0,0.45);"></span>'
        for _ in range(10))
    items = [
        ('turn', 'Turn upright', ''),
        ('plus', 'Add a slot', ''),
        ('minus', 'Remove the last slot', ''),
        ('lock', 'Lock in place', 'Ctrl will not move it'),
        ('eye_off', 'Hide', 'Edit layout brings it back'),
        ('reset', 'Back to its default place', ''),
    ]
    mi = ''.join(f'''<button role="menuitem" style="height: 32px; padding: 0 10px; border: 0; border-radius: 4px; background: {'rgba(224,176,104,0.14)' if i == 0 else 'transparent'}; display: flex; align-items: center; gap: 10px; font-size: 12.5px; font-weight: 600; cursor: pointer; text-align: left;">{svg_icon(ICONS[ic], 13, '#E9D3AE', 2.2)}<span style="flex: 1;">{label}</span><span style="font-size: 10.5px; color: {MUTED}; font-weight: 500;">{note}</span></button>'''
                 for i, (ic, label, note) in enumerate(items))
    menu = f'''<div role="menu" aria-label="Main bar" style="position: absolute; left: 250px; top: 120px; width: 280px; box-sizing: border-box; padding: 4px; border-radius: 6px; border: 1px solid {TRIM}; background: rgba(24,19,14,0.98); box-shadow: 0 12px 30px rgba(0,0,0,0.6); display: flex; flex-direction: column;">
      {section_label('MAIN BAR')}
      {mi}
      <span style="height: 1px; margin: 4px 6px; background: rgba(201,160,98,0.25);"></span>
      <button role="menuitem" style="height: 32px; padding: 0 10px; border: 0; border-radius: 4px; background: transparent; display: flex; align-items: center; gap: 10px; font-size: 12.5px; font-weight: 600; cursor: pointer; text-align: left;">{svg_icon(ICONS['sliders'], 13, '#E9D3AE', 2.2)}<span style="flex: 1;">Open Edit layout…</span><span style="font-size: 10.5px; color: {MUTED}; font-weight: 500;">keys, new bars</span></button>
    </div>'''

    def toggle(on=True):
        return (f'<span style="width: 34px; height: 18px; border-radius: 999px; background: {"rgba(224,176,104,0.9)" if on else "rgba(255,255,255,0.15)"}; position: relative; flex: none;">'
                f'<span style="position: absolute; top: 2px; {"right" if on else "left"}: 2px; width: 14px; height: 14px; border-radius: 50%; background: #1A140D;"></span></span>')
    opts = f'''<div style="position: absolute; right: 24px; top: 24px; width: 330px; box-sizing: border-box; border-radius: 6px; border: 1px solid {TRIM}; background: {PANEL}; box-shadow: 0 12px 30px rgba(0,0,0,0.5);">
    <div style="height: 40px; padding: 0 14px; display: flex; align-items: center; gap: 8px; border-bottom: 1px solid {TRIM_SOFT}; font-size: 12px; color: {SUB};">Options › Controls › <strong style="color: {TEXT}; font-weight: 600;">Moving the interface</strong></div>
    <div style="padding: 10px 14px; display: flex; flex-direction: column; gap: 12px;">
      <div style="display: flex; align-items: center; gap: 10px;"><span style="flex: 1; display: flex; flex-direction: column; gap: 2px;"><span style="font-size: 12.5px; font-weight: 600; display: flex; gap: 6px; align-items: center;">Quick move {OPENAC}</span><span style="font-size: 10.5px; color: {MUTED};">Hold a key and drag any piece, no unlocking</span></span>{toggle()}</div>
      <div style="display: flex; align-items: center; gap: 10px;"><span style="flex: 1; font-size: 12.5px; font-weight: 600;">Hold</span><span style="padding: 3px 10px; border-radius: 4px; border: 1px solid rgba(244,235,220,0.5); font-size: 11.5px; font-weight: 700;">Ctrl</span><button style="height: 26px; padding: 0 10px; border-radius: 4px; border: 1px solid rgba(201,160,98,0.6); background: rgba(0,0,0,0.35); font-size: 11.5px; font-weight: 600; cursor: pointer;">Change</button></div>
      <div style="display: flex; align-items: center; gap: 10px;"><span style="flex: 1; display: flex; flex-direction: column; gap: 2px;"><span style="font-size: 12.5px; font-weight: 600;">Snap to edges and other pieces</span><span style="font-size: 10.5px; color: {MUTED};">Hold Shift as well to place freely</span></span>{toggle()}</div>
      <div style="display: flex; align-items: center; gap: 10px;"><span style="flex: 1; display: flex; flex-direction: column; gap: 2px;"><span style="font-size: 12.5px; font-weight: 600;">Lock the interface</span><span style="font-size: 10.5px; color: {MUTED};">Retail's lock; quick move still works</span></span>{toggle()}</div>
    </div>
  </div>'''
    inner = f'''
  <div style="position: absolute; left: 24px; top: 70px; box-sizing: border-box; padding: 6px; border: 2px solid {ACCENT}; border-radius: 6px; background: rgba(224,176,104,0.1); display: flex; gap: 3px;">{slots}</div>
  <span style="position: absolute; left: 24px; top: 46px; display: flex; align-items: center; gap: 4px; padding: 1px 6px; border-radius: 3px; background: rgba(20,16,12,0.85); font-size: 10px; font-weight: 600;">{svg_icon(ICONS['move'], 10, ACCENT, 2.2)}Main bar</span>
  {menu}
  {opts}
  {caption(24, 360, 520, "Ctrl + right-click a piece while Ctrl is held: the quick edits for that piece (a bar turns, gains or loses slots; anything can be locked, hidden or sent back). Keys per slot and new bars stay in Edit layout. The key is set in Options › Controls, beside retail's lock.")}'''
    write('QuickMoveMenu.dc.html', page('Quick move: the piece menu', W, H, world(W, H, inner)))


# ---------------- plugin hooks ----------------
def plugin_card(name, v, lines, action=None):
    act = f'<button style="align-self: flex-start; height: 24px; padding: 0 8px; border-radius: 4px; border: 1px solid rgba(201,160,98,0.5); background: rgba(0,0,0,0.3); font-size: 11px; font-weight: 600; cursor: pointer;">{action}</button>' if action else ''
    vv = verdict(v) if v else ''
    ls = ''.join(f'<span style="font-size: 11px; color: {SUB}; line-height: 1.45;">{l}</span>' for l in lines)
    return f'''<div style="padding: 8px 10px; border-radius: 4px; border: 1px solid rgba(201,160,98,0.3); background: rgba(0,0,0,0.25); display: flex; flex-direction: column; gap: 4px;">
      <div style="display: flex; align-items: center; gap: 6px;">{svg_icon(ICONS['plugin'], 12, SUB, 2)}<span style="flex: 1; font-size: 12px; font-weight: 600;">{name}</span>{vv}</div>
      {ls}{act}
    </div>'''


def hook_inspect():
    W, H = 360, 640
    tabs = ''.join(f'<button role="tab" aria-selected="{"true" if t == "Overview" else "false"}" style="height: 30px; padding: 0 10px; border: 0; border-bottom: 2px solid {ACCENT if t == "Overview" else "transparent"}; background: transparent; font-size: 12px; font-weight: 600; color: {TEXT if t == "Overview" else SUB}; cursor: pointer;">{t}</button>' for t in ('Overview', 'Spells', 'Needs', 'Details'))

    def row(k, v, extra=''):
        return f'<div style="display: flex; justify-content: space-between; font-size: 12px; padding: 5px 0; border-bottom: 1px solid rgba(201,160,98,0.12);"><span style="color: {SUB};">{k}</span><span style="font-weight: 600; font-variant-numeric: tabular-nums;{extra}">{v}</span></div>'
    inner = f'''{header('shield', 'Gold Scale Hauberk', 'Armor · Workmanship 8 · Gold')}
  <div role="tablist" style="flex: none; display: flex; gap: 2px; padding: 0 8px; border-bottom: 1px solid {TRIM_SOFT};">{tabs}</div>
  <div style="flex: 1; min-height: 0; overflow: hidden; padding: 10px 12px; display: flex; flex-direction: column; gap: 8px;">
    <div style="display: flex; flex-direction: column; gap: 6px;">
      {section_label('FROM YOUR PLUGINS', 'plugin')}
      {plugin_card('Looter', KEEP, ['Profile “Mule run”, rule “Armor, AL 600+, wk 7+”.', 'Armor level 612 ≥ 600 · workmanship 8 ≥ 7'], 'Change the rule…')}
      {plugin_card('Value Check', None, ['Seen for 40,000 to 55,000 in trade chat this week.'])}
    </div>
    <div style="display: flex; flex-direction: column;">
      {row('Armor level', '612 ▲', ' color: #7FD08A;')}
      {row('Value', '12,400')}
      {row('Burden', '1,250')}
      {row('Covers', 'Chest, abdomen, upper arms')}
      {row('Slashing / piercing', '1.00 / 0.80')}
      {row('Magic', 'Major Invulnerability, +2 more')}
    </div>
  </div>'''
    body = f'<div style="width: {W}px; height: {H}px; background: #3B4A2E;">{window(W, H, inner)}</div>'
    write('HookInspect.dc.html', page('Plugin hook: inspect', W, H, body))


def hook_loot():
    W, H = 400, 600
    items = [
        ('shield', 'Gold Scale Hauberk', 'Armor · wk 8 · 12,400', KEEP),
        ('sword', 'Fire Spadone', 'Weapon · wk 6 · 8,100', SALV),
        ('gem', 'Black Opal', 'Gem · 2,500', SELL),
        ('coin', 'Pyreals', 'x 1,240', KEEP),
        ('gem', 'Small Ruby', 'Gem · 120', None),
        ('sword', 'Yari', 'Weapon · wk 3 · 300', None),
        ('shield', 'Leather Leggings', 'Armor · wk 2 · 90', None),
        ('sword', 'Iron Dagger', 'Weapon · wk 5 · 1,900', SALV),
        ('gem', 'Prismatic Taper', 'x 12 · 60', KEEP),
    ]
    rows = ''.join(f'''<div style="flex: none; height: 44px; padding: 0 6px; border-radius: 4px; display: flex; align-items: center; gap: 10px; {'opacity: 0.6;' if v is None else ''}">
        <span style="width: 32px; height: 32px; flex: none; border-radius: 4px; background: rgba(0,0,0,0.45); display: flex; align-items: center; justify-content: center;">{svg_icon(ICONS[ic], 16, '#E9D3AE')}</span>
        <span style="flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px;"><span style="font-size: 12.5px; font-weight: 600;">{n}</span><span style="font-size: 10.5px; color: {MUTED}; font-variant-numeric: tabular-nums;">{s}</span></span>
        {verdict(v) if v else f'<span style="font-size: 10px; color: {MUTED};">leave</span>'}
      </div>''' for ic, n, s, v in items)
    inner = f'''{header('down', 'Corpse of Drudge Skulker', '9 items')}
  <div style="flex: none; margin: 8px 12px 0; padding: 6px 10px; border-radius: 4px; border: 1px solid rgba(201,160,98,0.3); background: rgba(0,0,0,0.25); display: flex; align-items: center; gap: 8px; font-size: 11px; color: {SUB};">{svg_icon(ICONS['plugin'], 12, SUB)}<span style="flex: 1;"><strong style="color: {TEXT}; font-weight: 600;">Looter</strong> · 4 keep · 2 salvage · 1 sell · 2 leave</span></div>
  <div style="flex: 1; min-height: 0; padding: 8px 8px 4px; display: flex; flex-direction: column; gap: 2px; overflow: hidden;">{rows}</div>
  <div style="flex: none; height: 56px; padding: 0 12px; border-top: 1px solid rgba(201,160,98,0.3); display: flex; align-items: center; gap: 8px;">
    <span style="flex: 1; font-size: 10.5px; color: {SUB};">Burden 61% → 74%</span>
    {btn('Loot all')}
    {btn(svg_icon(ICONS['plugin'], 12, '#1A140D') + 'Loot 4 kept', True, 'display: flex; align-items: center; gap: 6px;')}
  </div>'''
    body = f'<div style="width: {W}px; height: {H}px; background: #3B4A2E;">{window(W, H, inner)}</div>'
    write('HookLoot.dc.html', page('Plugin hook: loot window', W, H, body))


def hook_menu():
    W, H = 480, 420

    def mi(ic, label, note='', on=False):
        return f'''<button role="menuitem" style="height: 30px; padding: 0 10px; border: 0; border-radius: 4px; background: {'rgba(224,176,104,0.14)' if on else 'transparent'}; display: flex; align-items: center; gap: 10px; font-size: 12.5px; font-weight: 600; cursor: pointer; text-align: left;">{svg_icon(ICONS[ic], 13, '#E9D3AE', 2.2)}<span style="flex: 1;">{label}</span><span style="font-size: 10.5px; color: {MUTED}; font-weight: 500;">{note}</span></button>'''
    menu = f'''<div role="menu" aria-label="Fire Spadone" style="position: absolute; left: 150px; top: 40px; width: 280px; box-sizing: border-box; padding: 4px; border-radius: 6px; border: 1px solid {TRIM}; background: rgba(24,19,14,0.99); box-shadow: 0 12px 30px rgba(0,0,0,0.6); display: flex; flex-direction: column;">
      {mi('search', 'Inspect')}
      {mi('down', 'Loot', 'double-click')}
      {mi('chevron', 'Loot into', 'Main pack…')}
      <span style="height: 1px; margin: 4px 6px; background: rgba(201,160,98,0.25);"></span>
      {section_label('LOOTER', 'plugin')}
      {mi('check', 'Keep items like this…', '', True)}
      {mi('cross', 'Never loot items like this')}
      {section_label('VALUE CHECK', 'plugin')}
      {mi('coin', 'Recent prices…')}
    </div>'''
    row = f'''<div style="position: absolute; left: 20px; top: 20px; width: 400px; height: 44px; box-sizing: border-box; padding: 0 6px; border-radius: 4px; border: 1px solid {ACCENT}; background: rgba(224,176,104,0.12); display: flex; align-items: center; gap: 10px; color: {TEXT}; font-family: Inter, system-ui, sans-serif;">
      <span style="width: 32px; height: 32px; border-radius: 4px; background: rgba(0,0,0,0.45); display: flex; align-items: center; justify-content: center;">{svg_icon(ICONS['sword'], 16, '#E9D3AE')}</span>
      <span style="flex: 1; font-size: 12.5px; font-weight: 600;">Fire Spadone</span>{verdict(SALV)}
    </div>'''
    body = f'''<div style="width: {W}px; height: {H}px; position: relative; background: #3B4A2E; font-family: Inter, system-ui, sans-serif; color: {TEXT};">{row}{menu}
  <p style="position: absolute; left: 20px; bottom: 16px; width: 440px; margin: 0; font-size: 11px; color: rgba(255,255,255,0.85); line-height: 1.5;">A plugin can add its own entries to an item's right-click menu (and the inventory's More menu), under its own name. They run the plugin, never the client.</p></div>'''
    write('HookMenu.dc.html', page('Plugin hook: item menu', W, H, body))


def hook_options():
    W, H = 1000, 620
    nav = ['Interface', 'Gameplay', 'Social', 'Chat', 'Controls', 'Graphics', 'Sound', 'Camera']
    navh = ''.join(f'<button style="height: 32px; padding: 0 12px; border: 0; border-radius: 4px; background: transparent; font-size: 12.5px; font-weight: 600; color: {SUB}; text-align: left; cursor: pointer;">{n}</button>' for n in nav)
    plug = ''.join(f'<button aria-current="{"page" if n == "Looter" else "false"}" style="height: 32px; padding: 0 12px; border: 0; border-radius: 4px; background: {"rgba(224,176,104,0.16)" if n == "Looter" else "transparent"}; font-size: 12.5px; font-weight: 600; color: {TEXT if n == "Looter" else SUB}; text-align: left; cursor: pointer; display: flex; align-items: center; gap: 8px;">{svg_icon(ICONS["plugin"], 12, ACCENT if n == "Looter" else SUB)}{n}</button>' for n in ('Looter', 'Value Check'))

    def toggle(on=True):
        return (f'<span style="width: 34px; height: 18px; border-radius: 999px; background: {"rgba(224,176,104,0.9)" if on else "rgba(255,255,255,0.15)"}; position: relative; flex: none;">'
                f'<span style="position: absolute; top: 2px; {"right" if on else "left"}: 2px; width: 14px; height: 14px; border-radius: 50%; background: #1A140D;"></span></span>')

    def trow(label, sub, on=True):
        return f'<div style="display: flex; align-items: center; gap: 12px; padding: 8px 0; border-bottom: 1px solid rgba(201,160,98,0.12);"><span style="flex: 1; display: flex; flex-direction: column; gap: 2px;"><span style="font-size: 12.5px; font-weight: 600;">{label}</span><span style="font-size: 10.5px; color: {MUTED};">{sub}</span></span>{toggle(on)}</div>'

    def slider(label, val, pct):
        return f'''<div style="display: flex; align-items: center; gap: 12px; padding: 8px 0; border-bottom: 1px solid rgba(201,160,98,0.12);"><span style="flex: 1; font-size: 12.5px; font-weight: 600;">{label}</span>
          <span style="width: 200px; height: 4px; border-radius: 2px; background: rgba(255,255,255,0.15); position: relative;"><span style="position: absolute; left: 0; top: 0; bottom: 0; width: {pct}%; border-radius: 2px; background: {ACCENT};"></span><span style="position: absolute; left: calc({pct}% - 7px); top: -5px; width: 14px; height: 14px; border-radius: 50%; background: {TEXT};"></span></span>
          <span style="width: 36px; text-align: right; font-size: 12px; font-weight: 600; font-variant-numeric: tabular-nums;">{val}</span></div>'''
    mats = ''.join(f'<span style="padding: 3px 10px; border-radius: 999px; border: 1px solid {"rgba(224,176,104,0.8)" if on else "rgba(201,160,98,0.3)"}; background: {"rgba(224,176,104,0.16)" if on else "transparent"}; font-size: 11.5px; font-weight: 600; color: {TEXT if on else MUTED};">{m}</span>' for m, on in (('Steel', True), ('Iron', True), ('Granite', True), ('Mahogany', False), ('Brass', False), ('Velvet', True)))
    inner = f'''<div style="flex: none; height: 48px; display: flex; align-items: center; gap: 12px; padding: 0 12px 0 16px; border-bottom: 1px solid {TRIM_SOFT};">
    <h1 style="margin: 0; font-size: 15px; font-weight: 700;">Options</h1>
    <span style="margin-left: 60px; width: 280px; height: 30px; box-sizing: border-box; padding: 0 10px; border-radius: 4px; border: 1px solid rgba(201,160,98,0.4); background: rgba(0,0,0,0.35); display: flex; align-items: center; gap: 8px; font-size: 12px; color: {MUTED};">{svg_icon(ICONS['search'], 13, MUTED)}Search all options</span>
    <span style="flex: 1;"></span>
    <button aria-label="Close" style="width: 30px; height: 30px; padding: 0; border: 0; border-radius: 4px; background: transparent; display: flex; align-items: center; justify-content: center; cursor: pointer;">{svg_icon(ICONS['close'], 16)}</button>
  </div>
  <div style="flex: 1; min-height: 0; display: flex;">
    <nav style="flex: none; width: 200px; padding: 10px 8px; box-sizing: border-box; border-right: 1px solid {TRIM_SOFT}; display: flex; flex-direction: column; gap: 1px;">
      {navh}
      <span style="height: 1px; margin: 8px 6px; background: rgba(201,160,98,0.25);"></span>
      {section_label('PLUGINS')}
      {plug}
    </nav>
    <div style="flex: 1; min-width: 0; padding: 16px 24px; display: flex; flex-direction: column; gap: 4px; overflow: hidden;">
      <div style="display: flex; align-items: baseline; gap: 10px;"><h2 style="margin: 0; font-size: 16px; font-weight: 700;">Looter</h2><span style="font-size: 11px; color: {MUTED};">version 2.3 · this page comes from the plugin, drawn in the Options style</span></div>
      <div style="display: flex; align-items: center; gap: 12px; padding: 10px 0; border-bottom: 1px solid rgba(201,160,98,0.12);"><span style="flex: 1; font-size: 12.5px; font-weight: 600;">Loot profile</span><span style="width: 200px; height: 30px; box-sizing: border-box; padding: 0 10px; border-radius: 4px; border: 1px solid rgba(201,160,98,0.5); background: rgba(0,0,0,0.35); display: flex; align-items: center; justify-content: space-between; font-size: 12px; font-weight: 600;">Mule run{svg_icon('M6 9l6 6 6-6', 12)}</span></div>
      {trow('Mark items in loot windows', 'Keep, Salvage, Sell beside each item')}
      {trow('Show the verdict on Inspect', 'A card under the item header')}
      {trow('Add entries to item menus', 'Keep items like this, Never loot')}
      {slider('Lowest workmanship to keep', '7', 70)}
      {slider('Lowest armor level to keep', '580', 58)}
      <div style="display: flex; align-items: center; gap: 12px; padding: 10px 0;"><span style="flex: none; width: 190px; font-size: 12.5px; font-weight: 600;">Salvage these materials</span><span style="display: flex; flex-wrap: wrap; gap: 6px;">{mats}</span></div>
    </div>
  </div>
  <div style="flex: none; height: 52px; padding: 0 16px; border-top: 1px solid {TRIM_SOFT}; display: flex; align-items: center; justify-content: flex-end; gap: 8px;">{btn('Defaults')}{btn('Revert')}{btn('Apply', True)}</div>'''
    body = f'<div style="width: {W}px; height: {H}px; background: #3B4A2E;">{window(W, H, inner)}</div>'
    write('HookOptions.dc.html', page('Plugin hook: options page', W, H, body))


recalls()
quick_move()
quick_menu()
hook_inspect()
hook_loot()
hook_menu()
hook_options()
print('more boards written')
