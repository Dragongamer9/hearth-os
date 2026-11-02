extends Control
## MechWarrior-style HUD: compass tape, bracket reticle with range readout,
## turret convergence circle, throttle and heat gauges, weapon status,
## component paper doll, legs-vs-torso indicator and status messages.

const CYAN := Color(0.55, 0.9, 1.0, 0.92)
const CYAN_DIM := Color(0.55, 0.9, 1.0, 0.35)
const CYAN_BG := Color(0.02, 0.08, 0.1, 0.45)
const AMBER := Color(1.0, 0.68, 0.2, 0.95)
const RED := Color(1.0, 0.28, 0.2, 0.95)
const WHITE := Color(0.95, 0.98, 1.0, 0.95)

var mech
var rig
var game
var player_input
var show_help := false
var _feed: Array = []   # [[text, time_left]]
var _font: Font
var _msg := ""
var _msg_t := 0.0
var _time := 0.0


func _ready() -> void:
	set_anchors_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	_font = ThemeDB.fallback_font
	Net.game_event.connect(_on_event)


func _on_event(text: String) -> void:
	_feed.append([text, 6.0])
	if _feed.size() > 5:
		_feed.pop_front()


func flash_message(text: String) -> void:
	_msg = text
	_msg_t = 2.0


func _process(delta: float) -> void:
	# Keep the HUD the size of the window (a Control under a CanvasLayer
	# doesn't follow window resizes on its own).
	# Scale the whole HUD with the view, so split-screen quarters stay readable.
	var vs := get_viewport_rect().size
	var sc := clampf(vs.y / 900.0, 0.5, 1.3)
	if scale.x != sc or size != vs / sc:
		position = Vector2.ZERO
		scale = Vector2(sc, sc)
		size = vs / sc
	_time += delta
	for f in _feed:
		f[1] -= delta
	while not _feed.is_empty() and _feed[0][1] <= 0.0:
		_feed.pop_front()
	_msg_t = maxf(_msg_t - delta, 0.0)
	queue_redraw()


func _text(pos: Vector2, text: String, fsize: int, col: Color, align: HorizontalAlignment = HORIZONTAL_ALIGNMENT_LEFT, width: float = -1.0) -> void:
	draw_string_outline(_font, pos, text, align, width, fsize, 4, Color(0, 0, 0, 0.55))
	draw_string(_font, pos, text, align, width, fsize, col)


func _panel(r: Rect2) -> void:
	draw_rect(r, CYAN_BG)
	var c := 8.0
	for corner in [r.position, Vector2(r.end.x, r.position.y), r.end, Vector2(r.position.x, r.end.y)]:
		var sx := 1.0 if corner.x == r.position.x else -1.0
		var sy := 1.0 if corner.y == r.position.y else -1.0
		draw_line(corner, corner + Vector2(c * sx, 0), CYAN, 2.0)
		draw_line(corner, corner + Vector2(0, c * sy), CYAN, 2.0)


func _draw() -> void:
	var vs := size
	var c := vs * 0.5
	if mech == null or not is_instance_valid(mech) or rig == null or rig.camera == null or mech.turret == null:
		var who: String = player_input.player_name if player_input != null else ""
		_text(Vector2(0, c.y), "%s  -  JOINING MATCH..." % who, 28, CYAN, HORIZONTAL_ALIGNMENT_CENTER, vs.x)
		return
	var cam: Camera3D = rig.camera
	_draw_tags(cam)
	_draw_scores(vs)
	if mech.dead:
		draw_rect(Rect2(Vector2.ZERO, vs), Color(0.15, 0.02, 0.0, 0.45))
		_text(Vector2(0, c.y - 20), "MECH DESTROYED", 40, RED, HORIZONTAL_ALIGNMENT_CENTER, vs.x)
		_text(Vector2(0, c.y + 24), "Respawning in %d" % ceili(maxf(mech.respawn_in, 0.0)) if mech.respawn_in < 100.0 else "Respawning...", 22, WHITE, HORIZONTAL_ALIGNMENT_CENTER, vs.x)
		return
	_draw_compass(Vector2(c.x, 34))
	_draw_reticle(c, cam)
	_draw_throttle(Vector2(46, vs.y - 330))
	_draw_heat(Vector2(vs.x - 78, vs.y - 330))
	_draw_weapons(Rect2(vs.x - 330, vs.y - 128, 230, 96))
	_draw_doll(Rect2(100, vs.y - 168, 150, 136))
	_draw_torso(Vector2(c.x, vs.y - 78))
	_draw_status(c)
	if show_help:
		_draw_help(vs)
	else:
		var hint := "H - CONTROLS" if player_input == null or player_input.is_kbm() else "VIEW - CONTROLS"
		_text(Vector2(vs.x - 170, 30), hint, 13, CYAN_DIM)


# ---------------------------------------------------------------- multiplayer overlays

func _draw_tags(cam: Camera3D) -> void:
	if game == null:
		return
	var pvp: bool = Net.mode == "pvp"
	for id in game.mechs:
		var m = game.mechs[id]
		if m == mech or m.dead or not is_instance_valid(m):
			continue
		var wp: Vector3 = m.global_position + Vector3(0, 5.5, 0)
		if cam.is_position_behind(wp):
			continue
		var dist := cam.global_position.distance_to(wp)
		if dist > 450.0:
			continue
		var sp := cam.unproject_position(wp) / scale.x
		var col := RED if pvp else CYAN
		_text(sp + Vector2(-80, -10), "%s  %dm" % [m.player_name, int(dist)], 14, col, HORIZONTAL_ALIGNMENT_CENTER, 160)
		draw_rect(Rect2(sp + Vector2(-30, -4), Vector2(60, 5)), Color(0, 0, 0, 0.5))
		draw_rect(Rect2(sp + Vector2(-30, -4), Vector2(60 * m.armor / 100.0, 5)), col)
		if pvp:
			draw_colored_polygon(PackedVector2Array([sp + Vector2(-6, 6), sp + Vector2(6, 6), sp + Vector2(0, 14)]), col)


func _draw_scores(vs: Vector2) -> void:
	var x := 20.0
	var y := 26.0
	var role: String = Net.role_label()
	_text(Vector2(x, y), "%s   %s   %s" % [mech.player_name, "PvP" if Net.mode == "pvp" else "CO-OP", role], 14, WHITE)
	y += 20
	if Net.mode == "pvp":
		var rows: Array = Net.roster.duplicate()
		rows.sort_custom(func(a, b): return int(Net.scores.get(a.id, {}).get("kills", 0)) > int(Net.scores.get(b.id, {}).get("kills", 0)))
		for e in rows.slice(0, 6):
			var sc: Dictionary = Net.scores.get(e.id, {"kills": 0, "deaths": 0})
			var me: bool = e.id == mech.mech_id
			_text(Vector2(x, y), "%-12s  %2d / %d" % [e.name, sc.kills, sc.deaths], 13, AMBER if me else CYAN)
			y += 17
	else:
		_text(Vector2(x, y), "TURRETS DESTROYED  %d" % Net.team_score, 13, CYAN)
		y += 17
	y += 6
	for f in _feed:
		_text(Vector2(x, y), f[0], 13, Color(AMBER, clampf(f[1], 0.0, 1.0)))
		y += 17


# ---------------------------------------------------------------- compass

func _draw_compass(top: Vector2) -> void:
	var w := clampf(size.x - 400.0, 260.0, 640.0)  # shrink in narrow split-screen views
	var px_per_deg := w / 120.0
	var view_deg := fposmod(-rad_to_deg(mech.aim_yaw), 360.0)
	var r := Rect2(top.x - w * 0.5, top.y - 14, w, 30)
	draw_rect(r, CYAN_BG)
	draw_line(Vector2(r.position.x, r.end.y), Vector2(r.end.x, r.end.y), CYAN_DIM, 1.0)
	var start := int(floor(view_deg - 60.0))
	for d in range(start, start + 121):
		if d % 5 != 0:
			continue
		var x := top.x + (float(d) - view_deg) * px_per_deg
		var big := d % 15 == 0
		draw_line(Vector2(x, r.end.y), Vector2(x, r.end.y - (10.0 if big else 5.0)), CYAN if big else CYAN_DIM, 1.5)
		if d % 30 == 0:
			var dd := posmod(d, 360)
			var lbl := str(dd)
			match dd:
				0: lbl = "N"
				90: lbl = "E"
				180: lbl = "S"
				270: lbl = "W"
			_text(Vector2(x - 20, r.position.y + 13), lbl, 14, CYAN, HORIZONTAL_ALIGNMENT_CENTER, 40)
	# Leg heading marker (triangle under the tape)
	var leg_deg := fposmod(-rad_to_deg(mech.rotation.y), 360.0)
	var off := wrapf(leg_deg - view_deg, -180.0, 180.0)
	var lx := top.x + clampf(off, -60.0, 60.0) * px_per_deg
	draw_colored_polygon(PackedVector2Array([Vector2(lx, r.end.y + 2), Vector2(lx - 7, r.end.y + 12), Vector2(lx + 7, r.end.y + 12)]), AMBER)
	draw_line(Vector2(top.x, r.position.y - 4), Vector2(top.x, r.end.y + 4), WHITE, 2.0)
	_text(Vector2(top.x - 30, r.end.y + 30), "%03d" % int(view_deg), 16, WHITE, HORIZONTAL_ALIGNMENT_CENTER, 60)


# ---------------------------------------------------------------- reticle

func _draw_reticle(c: Vector2, cam: Camera3D) -> void:
	var col := CYAN
	# Bracket crosshair
	var g := 22.0
	var l := 9.0
	for s in [Vector2(-1, -1), Vector2(1, -1), Vector2(1, 1), Vector2(-1, 1)]:
		var corner: Vector2 = c + s * g
		draw_line(corner, corner - Vector2(s.x * l, 0), col, 2.0)
		draw_line(corner, corner - Vector2(0, s.y * l), col, 2.0)
	draw_circle(c, 1.8, col)
	# Range to whatever is under the crosshair
	if mech.has_aim:
		var dist: float = cam.global_position.distance_to(mech.aim_target)
		var rtxt := "%dm" % int(dist) if dist < 1150.0 else "---"
		_text(c + Vector2(30, 34), rtxt, 14, CYAN)
	# Convergence circle: where the guns are actually pointing.
	var ap: Vector3 = mech.aim_point()
	if not cam.is_position_behind(ap):
		var sp := cam.unproject_position(ap)
		var on := sp.distance_to(c) < 14.0
		var rc := CYAN if on else AMBER
		draw_arc(sp, 9.0, 0.0, TAU, 28, rc, 2.0)
		for k in 4:
			var a := k * PI * 0.5 + PI * 0.25
			var d := Vector2(cos(a), sin(a))
			draw_line(sp + d * 11.0, sp + d * 16.0, rc, 2.0)


# ---------------------------------------------------------------- gauges

func _draw_throttle(top: Vector2) -> void:
	var r := Rect2(top, Vector2(30, 250))
	_panel(r.grow(6))
	var fwd_h := r.size.y * 0.72
	var zero_y := r.position.y + fwd_h
	for k in 11:
		var y := r.position.y + fwd_h * k / 10.0
		draw_line(Vector2(r.end.x, y), Vector2(r.end.x + (8.0 if k % 5 == 0 else 4.0), y), CYAN_DIM, 1.0)
	var th: float = mech.throttle
	var segs := 24
	for k in segs:
		var y0 := zero_y - fwd_h * (k + 1) / float(segs)
		var lit := float(k + 1) / float(segs) <= th + 0.001
		draw_rect(Rect2(r.position.x + 4, y0 + 1, r.size.x - 8, fwd_h / segs - 2), CYAN if lit else Color(CYAN, 0.08))
	var rev_h := r.size.y - fwd_h
	if th < 0.0:
		draw_rect(Rect2(r.position.x + 4, zero_y + 2, r.size.x - 8, rev_h * -th - 2), AMBER)
	draw_line(Vector2(r.position.x - 6, zero_y), Vector2(r.end.x + 6, zero_y), WHITE, 2.0)
	var spd: float = mech.speed
	var mf: float = mech.max_speed
	var mr: float = mech.reverse_speed
	var sy: float = zero_y - (spd / mf) * fwd_h if spd >= 0.0 else zero_y - (spd / mr) * rev_h
	draw_colored_polygon(PackedVector2Array([Vector2(r.position.x - 4, sy), Vector2(r.position.x - 14, sy - 6), Vector2(r.position.x - 14, sy + 6)]), WHITE)
	_text(Vector2(r.position.x - 10, r.position.y - 30), "%d" % int(absf(spd) * 3.6), 26, WHITE, HORIZONTAL_ALIGNMENT_LEFT)
	_text(Vector2(r.position.x - 10, r.position.y - 14), "KPH  " + ("TROT" if mech.trotting else "WALK"), 11, CYAN)


func _draw_heat(top: Vector2) -> void:
	var r := Rect2(top, Vector2(30, 250))
	_panel(r.grow(6))
	var heat: float = mech.heat
	var segs := 25
	for k in segs:
		var f := float(k + 1) / float(segs)
		var y0 := r.end.y - r.size.y * f
		var col := CYAN.lerp(AMBER, clampf((f - 0.4) / 0.35, 0.0, 1.0))
		if f > 0.8:
			col = RED
		var lit := f <= heat / 100.0 + 0.001
		draw_rect(Rect2(r.position.x + 4, y0 + 1, r.size.x - 8, r.size.y / segs - 2), col if lit else Color(col, 0.08))
	var y80 := r.end.y - r.size.y * 0.8
	draw_line(Vector2(r.position.x - 6, y80), Vector2(r.end.x + 6, y80), RED, 1.5)
	var hc := RED if heat > 80.0 or mech.overheated > 0.0 else (AMBER if heat > 50.0 else WHITE)
	_text(Vector2(r.position.x - 22, r.position.y - 30), "%d%%" % int(heat), 26, hc, HORIZONTAL_ALIGNMENT_RIGHT, 60)
	_text(Vector2(r.position.x - 22, r.position.y - 14), "HEAT", 11, CYAN, HORIZONTAL_ALIGNMENT_RIGHT, 60)


func _draw_weapons(r: Rect2) -> void:
	_panel(r)
	_text(r.position + Vector2(10, 18), "WEAPON GROUP 1", 12, CYAN_DIM)
	var names := ["AC/5  L", "AC/5  R"]
	var cd: float = clampf(mech._fire_cd / mech.fire_interval, 0.0, 1.0)
	for i in 2:
		var y := r.position.y + 40 + i * 24
		var is_ready: bool = mech.overheated <= 0.0 and (mech._next_muzzle == i or cd <= 0.0)
		var col := CYAN if is_ready else CYAN_DIM
		if mech.overheated > 0.0:
			col = RED
		_text(Vector2(r.position.x + 10, y + 4), names[i], 15, col)
		var bar := Rect2(r.position.x + 100, y - 7, 110, 10)
		draw_rect(bar, Color(CYAN, 0.1))
		var fill := 1.0 if is_ready else 1.0 - cd
		draw_rect(Rect2(bar.position, Vector2(bar.size.x * fill, bar.size.y)), col)
	var mode := "BRACED" if mech.braced else "MOBILE"
	_text(r.position + Vector2(10, r.size.y - 6), "STANCE: " + mode, 11, AMBER if mech.braced else CYAN_DIM)


func _draw_doll(r: Rect2) -> void:
	_panel(r)
	_text(r.position + Vector2(10, 18), "QUAD-07  CHIMERA", 12, CYAN_DIM)
	var c := r.get_center() + Vector2(0, 10)
	var a: float = mech.armor / 100.0
	var col := Color(0.4, 0.95, 0.6, 0.85)
	if a < 0.6:
		col = AMBER
	if a < 0.3:
		col = RED if fmod(_time, 0.6) < 0.4 else AMBER
	draw_rect(Rect2(c + Vector2(-18, -34), Vector2(36, 68)), Color(col, 0.25))
	draw_rect(Rect2(c + Vector2(-18, -34), Vector2(36, 68)), col, false, 1.5)
	draw_rect(Rect2(c + Vector2(-12, -24), Vector2(24, 30)), col, false, 1.5)
	for s in [Vector2(-1, -1), Vector2(1, -1), Vector2(-1, 1), Vector2(1, 1)]:
		var hip: Vector2 = c + Vector2(s.x * 18, s.y * 24)
		var knee: Vector2 = hip + Vector2(s.x * 22, s.y * 6 - 10)
		var foot: Vector2 = knee + Vector2(s.x * 12, s.y * 10 + 18)
		draw_polyline(PackedVector2Array([hip, knee, foot]), col, 4.0)
	_text(r.position + Vector2(10, r.size.y - 6), "ARMOR %d%%" % int(mech.armor), 11, col)


func _draw_torso(c: Vector2) -> void:
	var rad := 40.0
	draw_arc(c, rad, 0.0, TAU, 48, CYAN_DIM, 1.5)
	var legs_rel: float = mech.rotation.y - mech.aim_yaw
	var pts := PackedVector2Array()
	for p in [Vector2(-11, -17), Vector2(11, -17), Vector2(11, 17), Vector2(-11, 17)]:
		pts.append(c + p.rotated(-legs_rel))
	draw_colored_polygon(pts, Color(CYAN, 0.2))
	pts.append(pts[0])
	draw_polyline(pts, CYAN, 1.5)
	draw_line(c, c + Vector2(0, -26).rotated(-legs_rel), CYAN, 2.0)
	var torso_rel: float = mech.rotation.y + mech.turret.rotation.y - mech.aim_yaw
	draw_line(c, c + Vector2(0, -rad).rotated(-torso_rel), AMBER, 3.0)
	_text(c + Vector2(-50, rad + 20), "TORSO %+04d" % int(rad_to_deg(wrapf(mech.turret.rotation.y, -PI, PI))), 12, CYAN_DIM, HORIZONTAL_ALIGNMENT_CENTER, 100)


func _draw_status(c: Vector2) -> void:
	var status := ""
	var col := AMBER
	if mech.overheated > 0.0:
		status = "SHUTDOWN - OVERHEAT  %.1f" % mech.overheated
		col = RED if fmod(_time, 0.5) < 0.3 else AMBER
	elif mech.braced and mech.brace_amount < 1.0:
		status = "BRACING"
	elif mech.braced:
		status = "SIEGE MODE"
	elif mech.brace_amount > 0.0:
		status = "UNBRACING"
	elif mech.heat > 80.0:
		status = "HEAT CRITICAL"
		col = RED
	if _msg_t > 0.0:
		_text(Vector2(c.x - 200, 120), _msg, 18, CYAN, HORIZONTAL_ALIGNMENT_CENTER, 400)
	if status != "":
		_text(Vector2(c.x - 250, c.y - 70), status, 22, col, HORIZONTAL_ALIGNMENT_CENTER, 500)


func _draw_help(vs: Vector2) -> void:
	var lines := [
		"W / S     throttle up / down",
		"A / D     turn legs",
		"X         full stop",
		"MOUSE     aim torso",
		"LMB       fire autocannons",
		"B         brace / siege mode",
		"C         center view on legs",
		"V         cockpit / chase camera",
		"WHEEL     zoom chase camera",
		"F2        graphics quality",
		"F3        seat info",
		"ESC       pause",
	]
	if player_input != null and not player_input.is_kbm():
		lines = [
			"L-STICK   throttle / turn legs",
			"R-STICK   aim torso",
			"LT        precision aim",
			"RT / RB   fire autocannons",
			"B         brace / siege mode",
			"X         full stop",
			"Y         cockpit / chase camera",
			"L3        center view on legs",
			"D-PAD     zoom chase camera",
			"XBOX      seat info",
			"MENU      pause",
		]
	var r := Rect2(vs.x - 330, 70, 300, 26 + lines.size() * 20)
	_panel(r)
	for i in lines.size():
		_text(r.position + Vector2(12, 24 + i * 20), lines[i], 14, CYAN)
