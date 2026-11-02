extends Node
## Controls for ONE local player, read from one device: keyboard + mouse, or a
## single gamepad. The mech and camera read these instead of the global Input
## singleton, which is what makes split-screen (and per-seat input) work.
##
## Gamepad layout (Xbox):
##   Left stick   Y = throttle up/down, X = turn legs
##   Right stick  aim torso        LT (hold) = precision aim
##   RT / RB      fire             B = brace / siege mode
##   X            full stop        Y = cockpit / chase camera
##   L3 (click)   center on legs   D-pad up/down = zoom
##   View/Back    controls help    Menu/Start = pause

signal view_toggled
signal zoom_changed(direction: int)
signal help_toggled
signal pause_pressed
signal info_pressed

const KBM := -1
const DEADZONE := 0.18

var device := KBM
var player_name := "Player"

# Continuous values, refreshed every physics frame.
var throttle_axis := 0.0     # +1 = throttle up
var turn_axis := 0.0         # +1 = turn left
var aim_delta := Vector2.ZERO  # radians this frame: (yaw, pitch)
var fire := false
var enabled := true          # false while a menu is open

var mouse_sensitivity := 0.0025
var stick_aim_speed := 2.0   # rad/s at full deflection

var _mouse := Vector2.ZERO
var _queued := {}
var _pressed := {}


func is_kbm() -> bool:
	return device == KBM


func device_label() -> String:
	if device == KBM:
		return "Keyboard & Mouse"
	var n := Input.get_joy_name(device)
	return "%s (#%d)" % [n if n != "" else "Gamepad", device]


## One-shot actions: true for exactly one physics frame after the press.
func pressed(action: String) -> bool:
	return _pressed.get(action, false)


func _ready() -> void:
	process_physics_priority = -10  # update before the mech reads us


func _input(event: InputEvent) -> void:
	if not enabled:
		return
	if device == KBM:
		_kbm_event(event)
	else:
		_pad_event(event)


func _kbm_event(event: InputEvent) -> void:
	if event is InputEventMouseMotion and Input.mouse_mode == Input.MOUSE_MODE_CAPTURED:
		_mouse += event.relative
	elif event is InputEventMouseButton and event.pressed:
		if event.button_index == MOUSE_BUTTON_WHEEL_UP:
			zoom_changed.emit(1)
		elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			zoom_changed.emit(-1)
	elif event is InputEventKey and event.pressed and not event.echo:
		if event.is_action("brace"):
			_queued["brace"] = true
		elif event.is_action("full_stop"):
			_queued["stop"] = true
		elif event.is_action("center_torso"):
			_queued["center"] = true
		elif event.is_action("toggle_view"):
			view_toggled.emit()
		elif event.is_action("toggle_help"):
			help_toggled.emit()
		elif event.is_action("seat_info"):
			info_pressed.emit()
		elif event.keycode == KEY_ESCAPE:
			pause_pressed.emit()


func _pad_event(event: InputEvent) -> void:
	if event.device != device or not (event is InputEventJoypadButton) or not event.pressed:
		return
	match event.button_index:
		JOY_BUTTON_B:
			_queued["brace"] = true
		JOY_BUTTON_X:
			_queued["stop"] = true
		JOY_BUTTON_LEFT_STICK:
			_queued["center"] = true
		JOY_BUTTON_Y:
			view_toggled.emit()
		JOY_BUTTON_DPAD_UP:
			zoom_changed.emit(1)
		JOY_BUTTON_DPAD_DOWN:
			zoom_changed.emit(-1)
		JOY_BUTTON_BACK:
			help_toggled.emit()
		JOY_BUTTON_START:
			pause_pressed.emit()
		JOY_BUTTON_GUIDE:
			info_pressed.emit()


func _physics_process(delta: float) -> void:
	_pressed = _queued
	_queued = {}
	if not enabled:
		throttle_axis = 0.0
		turn_axis = 0.0
		aim_delta = Vector2.ZERO
		fire = false
		_mouse = Vector2.ZERO
		_pressed = {}
		return
	if device == KBM:
		throttle_axis = Input.get_axis("throttle_down", "throttle_up")
		turn_axis = Input.get_axis("turn_right", "turn_left")
		fire = Input.is_action_pressed("fire") and Input.mouse_mode == Input.MOUSE_MODE_CAPTURED
		aim_delta = -_mouse * mouse_sensitivity
		_mouse = Vector2.ZERO
	else:
		var ly := _dz(Input.get_joy_axis(device, JOY_AXIS_LEFT_Y))
		var lx := _dz(Input.get_joy_axis(device, JOY_AXIS_LEFT_X))
		var rx := _curve(_dz(Input.get_joy_axis(device, JOY_AXIS_RIGHT_X)))
		var ry := _curve(_dz(Input.get_joy_axis(device, JOY_AXIS_RIGHT_Y)))
		throttle_axis = -ly
		turn_axis = -lx
		var precise := Input.get_joy_axis(device, JOY_AXIS_TRIGGER_LEFT) > 0.4
		var spd := stick_aim_speed * (0.35 if precise else 1.0)
		aim_delta = Vector2(-rx, -ry) * spd * delta
		fire = Input.get_joy_axis(device, JOY_AXIS_TRIGGER_RIGHT) > 0.4 or Input.is_joy_button_pressed(device, JOY_BUTTON_RIGHT_SHOULDER)


func _dz(v: float) -> float:
	if absf(v) < DEADZONE:
		return 0.0
	return signf(v) * (absf(v) - DEADZONE) / (1.0 - DEADZONE)


func _curve(v: float) -> float:
	return signf(v) * pow(absf(v), 1.6)
