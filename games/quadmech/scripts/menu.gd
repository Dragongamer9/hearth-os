extends CanvasLayer
## Menus, all controller-navigable:
##  - Main: "press A / Enter to join" for up to 4 local players, then Play Local,
##    Host LAN, Join LAN, Mode (Co-op / PvP), Graphics, Seat Info, Quit
##  - LAN browser: games found on the network (and on this machine's other seats)
##  - Pause: Resume, Seat Info, Leave Match, Quit
##  - Seat info overlay (F3 / Xbox button): what this copy of the game can see,
##    for checking Hearth OS seat isolation

const CYAN := Color(0.55, 0.9, 1.0)
const AMBER := Color(1.0, 0.68, 0.2)
const MAX_LOCAL := 4
const KBM := -1

var main
var joined: Array = []   # [{device, name}]

var _root: Control
var _main_panel: Control
var _browser_panel: Control
var _pause_panel: Control
var _info_panel: PanelContainer
var _info_label: Label
var _slots: Array[Label] = []
var _status: Label
var _toast: Label
var _mode_btn: Button
var _gfx_btn: Button
var _host_list: VBoxContainer
var _search_label: Label
var _ip_edit: LineEdit
var _first_main: Button
var _first_pause: Button
var _auto_join := false
var _last_device := KBM
var _info_t := 0.0
var _toast_t := 0.0


func _ready() -> void:
	layer = 10
	_root = Control.new()
	_root.set_anchors_preset(Control.PRESET_FULL_RECT)
	_root.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_root.theme = _make_theme()
	add_child(_root)
	_build_main()
	_build_browser()
	_build_pause()
	_build_info()
	_toast = _label("", 26, AMBER)
	_toast.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_root.add_child(_toast)
	_toast.set_anchors_and_offsets_preset(Control.PRESET_TOP_WIDE)
	_toast.offset_top = 40
	Net.status_changed.connect(_on_status)
	Net.hosts_changed.connect(_refresh_hosts)
	joined = Net.local_players.duplicate(true)
	_refresh_slots()
	hide_all()


func is_open() -> bool:
	return _main_panel.visible or _browser_panel.visible or _pause_panel.visible


func hide_all() -> void:
	_main_panel.visible = false
	_browser_panel.visible = false
	_pause_panel.visible = false
	Net.stop_browsing()


func show_main() -> void:
	hide_all()
	_main_panel.visible = true
	Input.mouse_mode = Input.MOUSE_MODE_VISIBLE
	_refresh_buttons()
	_first_main.grab_focus()


func show_browser(auto: bool = false) -> void:
	hide_all()
	_auto_join = auto
	_browser_panel.visible = true
	Net.start_browsing()
	_refresh_hosts()
	if _host_list.get_child_count() == 0:
		_ip_edit.grab_focus()
	else:
		_host_list.get_child(0).grab_focus()


func show_pause() -> void:
	if not main.in_match or _pause_panel.visible:
		return
	_pause_panel.visible = true
	main.set_inputs_enabled(false)
	_first_pause.grab_focus()


func _resume() -> void:
	_pause_panel.visible = false
	main.set_inputs_enabled(true)


func toggle_info() -> void:
	_info_panel.visible = not _info_panel.visible


func show_message(t: String, secs: float) -> void:
	_toast.text = t
	_toast_t = secs


## Fills the local player list from command-line options (used by Hearth when
## it launches a seat directly into a match).
func join_from_args(args: Dictionary) -> void:
	joined.clear()
	var spec: String = args.get("players", "auto")
	var base: String = args.get("name", "")
	if spec == "auto" or spec == "":
		var pads := Input.get_connected_joypads()
		if pads.is_empty():
			_add_player(KBM, base)
		else:
			for d in pads.slice(0, MAX_LOCAL):
				_add_player(d, base)
	else:
		for tok in spec.split(","):
			tok = tok.strip_edges()
			if tok == "kbm":
				_add_player(KBM, base)
			elif tok.is_valid_int():
				_add_player(int(tok), base)
	if args.has("mode"):
		Net.mode = "pvp" if args.mode == "pvp" else "coop"
	Net.local_players = joined.duplicate(true)


# ---------------------------------------------------------------- input

func _input(event: InputEvent) -> void:
	if _info_panel.visible and event.is_action_pressed("seat_info"):
		toggle_info()
		get_viewport().set_input_as_handled()
		return
	if event is InputEventJoypadButton or event is InputEventJoypadMotion:
		_last_device = event.device
	elif event is InputEventKey or event is InputEventMouseButton:
		_last_device = KBM

	if _pause_panel.visible:
		if event is InputEventJoypadButton and event.pressed and event.button_index == JOY_BUTTON_START:
			_resume()
			get_viewport().set_input_as_handled()
		elif event is InputEventKey and event.pressed and event.keycode == KEY_ESCAPE:
			_resume()
			get_viewport().set_input_as_handled()
		return

	if _browser_panel.visible:
		if event.is_action_pressed("ui_cancel") and not _ip_edit.has_focus():
			show_main()
			get_viewport().set_input_as_handled()
		return

	if not _main_panel.visible:
		if event.is_action_pressed("seat_info"):
			toggle_info()
		return

	# Join / leave on the main screen.
	if event is InputEventJoypadButton and event.pressed:
		var d: int = event.device
		if not _is_joined(d) and event.button_index in [JOY_BUTTON_A, JOY_BUTTON_START]:
			_add_player(d)
			get_viewport().set_input_as_handled()
		elif _is_joined(d) and event.button_index == JOY_BUTTON_B and joined.size() > 1:
			_remove_player(d)
			get_viewport().set_input_as_handled()
	elif event is InputEventKey and event.pressed and not event.echo:
		if not _is_joined(KBM) and event.keycode in [KEY_ENTER, KEY_SPACE, KEY_KP_ENTER]:
			_add_player(KBM)
			get_viewport().set_input_as_handled()
		elif event.is_action("seat_info"):
			toggle_info()
	elif event is InputEventMouseButton and event.pressed and not _is_joined(KBM) and joined.size() < MAX_LOCAL:
		_add_player(KBM)


func _is_joined(d: int) -> bool:
	for p in joined:
		if p.device == d:
			return true
	return false


func _add_player(d: int, base: String = "") -> void:
	if joined.size() >= MAX_LOCAL or _is_joined(d):
		return
	var n := joined.size() + 1
	var nm := "P%d" % n
	if base != "":
		nm = base if n == 1 else "%s %d" % [base, n]
	joined.append({"device": d, "name": nm})
	_refresh_slots()


func _remove_player(d: int) -> void:
	for i in joined.size():
		if joined[i].device == d:
			joined.remove_at(i)
			break
	for i in joined.size():
		joined[i].name = "P%d" % (i + 1)
	_refresh_slots()


func _ensure_players() -> void:
	if joined.is_empty():
		_add_player(_last_device)
	Net.local_players = joined.duplicate(true)


func _process(delta: float) -> void:
	_toast_t -= delta
	_toast.visible = _toast_t > 0.0
	if _info_panel.visible:
		_info_t -= delta
		if _info_t <= 0.0:
			_info_t = 0.5
			_info_label.text = _seat_text()


# ---------------------------------------------------------------- building

func _make_theme() -> Theme:
	var t := Theme.new()
	t.default_font_size = 24
	var normal := StyleBoxFlat.new()
	normal.bg_color = Color(0.02, 0.06, 0.08, 0.75)
	normal.border_color = Color(CYAN, 0.35)
	normal.set_border_width_all(2)
	normal.set_content_margin_all(12)
	normal.content_margin_left = 22
	normal.content_margin_right = 22
	var focus := normal.duplicate() as StyleBoxFlat
	focus.bg_color = Color(0.05, 0.2, 0.26, 0.92)
	focus.border_color = CYAN
	focus.set_border_width_all(3)
	var hover := focus.duplicate() as StyleBoxFlat
	hover.border_color = Color(CYAN, 0.7)
	t.set_stylebox("normal", "Button", normal)
	t.set_stylebox("focus", "Button", focus)
	t.set_stylebox("hover", "Button", hover)
	t.set_stylebox("pressed", "Button", focus)
	t.set_color("font_color", "Button", Color(CYAN, 0.85))
	t.set_color("font_focus_color", "Button", Color.WHITE)
	t.set_color("font_hover_color", "Button", Color.WHITE)
	t.set_color("font_color", "Label", CYAN)
	var panel := StyleBoxFlat.new()
	panel.bg_color = Color(0.01, 0.025, 0.035, 0.82)
	panel.border_color = Color(CYAN, 0.25)
	panel.set_border_width_all(1)
	panel.set_content_margin_all(26)
	t.set_stylebox("panel", "PanelContainer", panel)
	var edit := normal.duplicate() as StyleBoxFlat
	t.set_stylebox("normal", "LineEdit", edit)
	t.set_stylebox("focus", "LineEdit", focus)
	t.set_color("font_color", "LineEdit", Color.WHITE)
	return t


func _label(text: String, fsize: int, col: Color = CYAN) -> Label:
	var l := Label.new()
	l.text = text
	l.add_theme_font_size_override("font_size", fsize)
	l.add_theme_color_override("font_color", col)
	l.add_theme_color_override("font_outline_color", Color(0, 0, 0, 0.7))
	l.add_theme_constant_override("outline_size", 6)
	return l


func _button(text: String, cb: Callable) -> Button:
	var b := Button.new()
	b.text = text
	b.custom_minimum_size = Vector2(380, 54)
	b.alignment = HORIZONTAL_ALIGNMENT_LEFT
	b.pressed.connect(cb)
	return b


func _panel(anchor: int) -> PanelContainer:
	var p := PanelContainer.new()
	_root.add_child(p)
	p.set_anchors_preset(anchor)
	p.grow_horizontal = Control.GROW_DIRECTION_BOTH
	p.grow_vertical = Control.GROW_DIRECTION_BOTH
	return p


func _build_main() -> void:
	_main_panel = Control.new()
	_main_panel.set_anchors_preset(Control.PRESET_FULL_RECT)
	_main_panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_root.add_child(_main_panel)

	var shade := ColorRect.new()
	shade.set_anchors_preset(Control.PRESET_LEFT_WIDE)
	shade.custom_minimum_size.x = 620
	shade.color = Color(0.0, 0.015, 0.02, 0.6)
	shade.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_main_panel.add_child(shade)

	var col := VBoxContainer.new()
	col.position = Vector2(70, 60)
	col.add_theme_constant_override("separation", 12)
	_main_panel.add_child(col)
	col.add_child(_label("QUAD MECH", 72, Color.WHITE))
	col.add_child(_label("HEARTH OS  MULTI-SEAT TEST", 22, AMBER))
	var gap := Control.new()
	gap.custom_minimum_size.y = 18
	col.add_child(gap)

	_first_main = _button("PLAY LOCAL  (split-screen)", _on_play_local)
	col.add_child(_first_main)
	col.add_child(_button("HOST LAN GAME", _on_host))
	col.add_child(_button("JOIN LAN GAME", _on_join))
	_mode_btn = _button("", _on_mode)
	col.add_child(_mode_btn)
	_gfx_btn = _button("", _on_gfx)
	col.add_child(_gfx_btn)
	col.add_child(_button("SEAT INFO", func(): toggle_info()))
	col.add_child(_button("QUIT", func(): get_tree().quit()))
	_status = _label("", 18, Color(CYAN, 0.7))
	col.add_child(_status)

	# Player slots, bottom right
	var slots := VBoxContainer.new()
	slots.add_theme_constant_override("separation", 6)
	_main_panel.add_child(slots)
	slots.add_child(_label("PLAYERS ON THIS SEAT", 20, AMBER))
	for i in MAX_LOCAL:
		var l := _label("", 22)
		slots.add_child(l)
		_slots.append(l)
	slots.add_child(_label("A / Enter to join    B to leave", 16, Color(CYAN, 0.6)))
	slots.set_anchors_and_offsets_preset(Control.PRESET_BOTTOM_RIGHT, Control.PRESET_MODE_MINSIZE, 60)
	slots.grow_horizontal = Control.GROW_DIRECTION_BEGIN
	slots.grow_vertical = Control.GROW_DIRECTION_BEGIN


func _build_browser() -> void:
	var p := _panel(Control.PRESET_CENTER)
	p.custom_minimum_size = Vector2(640, 520)
	_browser_panel = p
	var col := VBoxContainer.new()
	col.add_theme_constant_override("separation", 10)
	p.add_child(col)
	col.add_child(_label("JOIN LAN GAME", 40, Color.WHITE))
	_search_label = _label("Searching the network and this machine...", 18)
	col.add_child(_search_label)
	_host_list = VBoxContainer.new()
	_host_list.add_theme_constant_override("separation", 8)
	_host_list.size_flags_vertical = Control.SIZE_EXPAND_FILL
	col.add_child(_host_list)
	col.add_child(_label("Or enter an address:", 18))
	var row := HBoxContainer.new()
	col.add_child(row)
	_ip_edit = LineEdit.new()
	_ip_edit.placeholder_text = "192.168.1.20"
	_ip_edit.custom_minimum_size = Vector2(360, 50)
	row.add_child(_ip_edit)
	var go := _button("CONNECT", _on_connect_ip)
	go.custom_minimum_size.x = 200
	row.add_child(go)
	col.add_child(_button("BACK", func(): show_main()))


func _build_pause() -> void:
	var p := _panel(Control.PRESET_CENTER)
	p.custom_minimum_size = Vector2(460, 360)
	_pause_panel = p
	var col := VBoxContainer.new()
	col.add_theme_constant_override("separation", 10)
	p.add_child(col)
	col.add_child(_label("PAUSED", 40, Color.WHITE))
	col.add_child(_label("(the match keeps running for other players)", 15, Color(CYAN, 0.6)))
	_first_pause = _button("RESUME", func(): _resume())
	col.add_child(_first_pause)
	col.add_child(_button("SEAT INFO", func(): toggle_info()))
	col.add_child(_button("LEAVE MATCH", func(): main.leave_match()))
	col.add_child(_button("QUIT", func(): get_tree().quit()))


func _build_info() -> void:
	_info_panel = PanelContainer.new()
	_info_panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_info_panel.visible = false
	_root.add_child(_info_panel)
	_info_label = _label("", 16, Color(0.85, 0.95, 1.0))
	_info_panel.add_child(_info_label)
	_info_panel.set_anchors_and_offsets_preset(Control.PRESET_TOP_RIGHT, Control.PRESET_MODE_MINSIZE, 30)
	_info_panel.offset_top = 90
	_info_panel.grow_horizontal = Control.GROW_DIRECTION_BEGIN


# ---------------------------------------------------------------- actions

func _on_status(t: String) -> void:
	_status.text = t
	show_message(t, 2.0)


func _on_play_local() -> void:
	_ensure_players()
	main.start_match("local")


func _on_host() -> void:
	_ensure_players()
	main.start_match("host")


func _on_join() -> void:
	_ensure_players()
	show_browser()


func _on_mode() -> void:
	Net.set_mode("pvp" if Net.mode == "coop" else "coop")
	_refresh_buttons()


func _on_gfx() -> void:
	main.set_quality(not main.high_quality)
	_refresh_buttons()


func _on_connect_ip() -> void:
	var ip := _ip_edit.text.strip_edges()
	if ip != "":
		main.start_match("join", ip)


# ---------------------------------------------------------------- refresh

func _refresh_slots() -> void:
	for i in MAX_LOCAL:
		if i < joined.size():
			var d: int = joined[i].device
			var jn := Input.get_joy_name(d)
			var dev := "Keyboard & Mouse" if d == KBM else "%s (#%d)" % [jn if jn != "" else "Controller", d]
			_slots[i].text = "%s   %s" % [joined[i].name, dev]
			_slots[i].add_theme_color_override("font_color", Color.WHITE)
		else:
			_slots[i].text = "P%d   press A to join" % (i + 1)
			_slots[i].add_theme_color_override("font_color", Color(CYAN, 0.35))


func _refresh_buttons() -> void:
	_mode_btn.text = "MODE:  %s" % ("PvP DEATHMATCH" if Net.mode == "pvp" else "CO-OP vs TURRETS")
	_gfx_btn.text = "GRAPHICS:  %s" % ("HIGH" if main.high_quality else "LOW")


func _refresh_hosts() -> void:
	if not _browser_panel.visible:
		return
	for c in _host_list.get_children():
		c.queue_free()
	var ips: Array = Net.found_hosts.keys()
	_search_label.text = "Searching..." if ips.is_empty() else "Games found:"
	for ip in ips:
		var h: Dictionary = Net.found_hosts[ip]
		var where: String = "this machine" if ip == "127.0.0.1" else str(ip)
		var b := _button("%s   %s   %d players   (%s)" % [h.name, "PvP" if h.mode == "pvp" else "Co-op", h.players, where],
			func(): main.start_match("join", ip))
		_host_list.add_child(b)
	if _auto_join and not ips.is_empty():
		_auto_join = false
		main.start_match("join", ips[0])
	elif _host_list.get_child_count() > 0 and not _ip_edit.has_focus():
		_host_list.get_child(0).call_deferred("grab_focus")


func _seat_text() -> String:
	var L: PackedStringArray = []
	L.append("SEAT DIAGNOSTICS   (F3 / Xbox button)")
	L.append("Process id: %d" % OS.get_process_id())
	L.append("OS: %s   Display server: %s" % [OS.get_name(), DisplayServer.get_name()])
	var rd := RenderingServer.get_rendering_device()
	L.append("Renderer: %s  (%s)" % ["Vulkan / D3D12 (Forward+)" if rd != null else "OpenGL (Compatibility)", "High" if main.high_quality else "Low"])
	L.append("GPU: %s" % RenderingServer.get_video_adapter_name())
	L.append("Window: %s on screen %d of %d   FPS %d" % [str(DisplayServer.window_get_size()), DisplayServer.window_get_current_screen(), DisplayServer.get_screen_count(), Engine.get_frames_per_second()])
	for v in ["XDG_SEAT", "XDG_SESSION_ID", "XDG_VTNR", "DISPLAY", "WAYLAND_DISPLAY", "GAMESCOPE_WAYLAND_DISPLAY", "SteamAppId"]:
		var val := OS.get_environment(v)
		if val != "":
			L.append("%s=%s" % [v, val])
	L.append("")
	var pads := Input.get_connected_joypads()
	L.append("Controllers this copy can see: %d" % pads.size())
	for d in pads:
		var mine := ""
		for p in joined:
			if p.device == d:
				mine = "   <- " + p.name
		L.append("  #%d  %s%s" % [d, Input.get_joy_name(d), mine])
	L.append("")
	L.append("Network: %s   peer %d" % [Net.role_label(), Net.my_peer()])
	L.append("Players in match: %d" % Net.roster.size())
	for e in Net.roster:
		L.append("  %s  (peer %d)%s" % [e.name, e.peer, "  local" if int(e.peer) == Net.my_peer() else ""])
	return "\n".join(L)
