extends Node
## Multiplayer for the multi-seat test: LAN hosting/joining over ENet, LAN game
## discovery, the player roster, and sync of mech state, shots, damage and scores.
##
## Offline (local split-screen) uses the same code: Godot's offline peer makes
## this machine the authority, and call_local RPCs simply run locally.
##
## Authority: each machine moves its own mechs and reports their state; the host
## decides every hit (damage, kills, turret hp) and broadcasts the result.

signal roster_changed
signal status_changed(text: String)
signal game_event(text: String)
signal hosts_changed
signal disconnected

const PORT := 7777
const DISCOVERY_PORT := 7778
const MAX_PLAYERS := 16
const SHELL_DAMAGE := 4.0

var mode := "coop"            # "coop" or "pvp"
var roster: Array = []        # [{id, peer, idx, name}]
var local_players: Array = [] # [{device, name}] on this machine
var scores := {}              # id -> {kills, deaths}
var team_score := 0
var online := false
var found_hosts := {}         # ip -> {name, mode, players, time}
var game = null               # set by main: has mechs (id -> mech) and targets (Array)

var _udp: PacketPeerUDP
var _browsing := false
var _responding := false
var _browse_t := 0.0
var _host_name := ""


func _ready() -> void:
	multiplayer.peer_connected.connect(_on_peer_connected)
	multiplayer.peer_disconnected.connect(_on_peer_disconnected)
	multiplayer.connected_to_server.connect(_on_connected)
	multiplayer.connection_failed.connect(func(): _status("Could not connect to host"); disconnected.emit())
	multiplayer.server_disconnected.connect(func(): _status("Host left the game"); disconnected.emit())
	var user := OS.get_environment("USER")
	if user == "":
		user = OS.get_environment("USERNAME")
	_host_name = (user if user != "" else "Hearth") + "'s battle"


func is_authority() -> bool:
	return multiplayer.is_server()


func my_peer() -> int:
	return multiplayer.get_unique_id()


func role_label() -> String:
	if not online:
		return "Local"
	return "Host" if multiplayer.is_server() else "Client (peer %d)" % my_peer()


# ---------------------------------------------------------------- sessions

func start_local() -> void:
	leave()
	online = false
	_rebuild_local_roster()


func host_game() -> bool:
	leave()
	var peer := ENetMultiplayerPeer.new()
	var err := peer.create_server(PORT, MAX_PLAYERS)
	if err != OK:
		_status("Could not host (port %d busy?)" % PORT)
		return false
	multiplayer.multiplayer_peer = peer
	online = true
	_rebuild_local_roster()
	_start_responder()
	_status("Hosting on port %d" % PORT)
	return true


func join_game(ip: String) -> bool:
	leave()
	var peer := ENetMultiplayerPeer.new()
	var err := peer.create_client(ip, PORT)
	if err != OK:
		_status("Could not connect to %s" % ip)
		return false
	multiplayer.multiplayer_peer = peer
	online = true
	roster = []
	_status("Connecting to %s..." % ip)
	return true


func leave() -> void:
	stop_browsing()
	_stop_udp()
	_responding = false
	if multiplayer.multiplayer_peer != null and not (multiplayer.multiplayer_peer is OfflineMultiplayerPeer):
		multiplayer.multiplayer_peer.close()
	multiplayer.multiplayer_peer = OfflineMultiplayerPeer.new()
	online = false
	roster = []
	scores = {}
	team_score = 0


func _rebuild_local_roster() -> void:
	var me := my_peer()
	var keep: Array = []
	for e in roster:
		if e.peer != me:
			keep.append(e)
	for i in local_players.size():
		keep.append({"id": "%d:%d" % [me, i], "peer": me, "idx": i, "name": local_players[i].name})
	roster = keep
	for e in roster:
		if not scores.has(e.id):
			scores[e.id] = {"kills": 0, "deaths": 0}
	roster_changed.emit()


func _on_connected() -> void:
	_status("Connected")
	var names: Array = []
	for p in local_players:
		names.append(p.name)
	_register.rpc_id(1, names)


func _on_peer_connected(_id: int) -> void:
	pass


func _on_peer_disconnected(id: int) -> void:
	if not multiplayer.is_server():
		return
	var keep: Array = []
	for e in roster:
		if e.peer != id:
			keep.append(e)
		else:
			game_event.emit("%s left" % e.name)
	roster = keep
	_broadcast_roster()


@rpc("any_peer", "reliable")
func _register(names: Array) -> void:
	if not multiplayer.is_server():
		return
	var sender := multiplayer.get_remote_sender_id()
	var keep: Array = []
	for e in roster:
		if e.peer != sender:
			keep.append(e)
	for i in names.size():
		var id := "%d:%d" % [sender, i]
		keep.append({"id": id, "peer": sender, "idx": i, "name": str(names[i])})
		if not scores.has(id):
			scores[id] = {"kills": 0, "deaths": 0}
		game_event.emit("%s joined" % names[i])
	roster = keep
	_broadcast_roster()
	# Bring the newcomer up to date on turret damage.
	if game != null:
		var hps: Array = []
		for t in game.targets:
			hps.append(t.hp)
		_target_states.rpc_id(sender, hps)


func _broadcast_roster() -> void:
	_set_roster.rpc(roster, mode, scores, team_score)


@rpc("authority", "call_local", "reliable")
func _set_roster(r: Array, m: String, s: Dictionary, ts: int) -> void:
	roster = r
	mode = m
	scores = s
	team_score = ts
	roster_changed.emit()


func set_mode(m: String) -> void:
	mode = m
	if online and multiplayer.is_server():
		_broadcast_roster()


func spawn_point_for(id: String) -> Array:
	var n := maxi(roster.size(), 1)
	var slot := 0
	for i in roster.size():
		if roster[i].id == id:
			slot = i
	var ang := TAU * slot / float(maxi(n, 4)) + randf_range(-0.2, 0.2)
	var r := 14.0 + randf() * 6.0
	var pos := Vector3(cos(ang) * r, 0.0, sin(ang) * r)
	if game != null:
		pos.y = game.terrain.height_at(pos.x, pos.z) + 3.4
	return [pos, -ang - PI * 0.5]


# ---------------------------------------------------------------- mech sync

func send_state(id: String, pos: Vector3, yaw: float, spd: float, yr: float, tur: float, pitch: float, brc: bool, heat: float) -> void:
	if online:
		_state.rpc(id, pos, yaw, spd, yr, tur, pitch, brc, heat)


@rpc("any_peer", "unreliable_ordered")
func _state(id: String, pos: Vector3, yaw: float, spd: float, yr: float, tur: float, pitch: float, brc: bool, heat: float) -> void:
	var m = _mech(id)
	if m != null and not m.is_local:
		m.apply_net_state(pos, yaw, spd, yr, tur, pitch, brc, heat)


func send_fire(id: String, muzzle: int, origin: Vector3, dir: Vector3) -> void:
	if online:
		_fired.rpc(id, muzzle, origin, dir)


@rpc("any_peer", "reliable")
func _fired(id: String, muzzle: int, origin: Vector3, dir: Vector3) -> void:
	var m = _mech(id)
	if m != null and not m.is_local:
		m.shoot(muzzle, origin, dir)


func send_respawn(id: String, pos: Vector3, yaw: float) -> void:
	_respawned.rpc(id, pos, yaw)


@rpc("any_peer", "call_local", "reliable")
func _respawned(id: String, pos: Vector3, yaw: float) -> void:
	var m = _mech(id)
	if m != null:
		m.respawn_at(pos, yaw)


# ---------------------------------------------------------------- damage (host decides)

func mech_hit(victim: String, shooter: String, dmg: float) -> void:
	if not multiplayer.is_server():
		return
	if mode == "coop" and victim != shooter:
		return  # no friendly fire in co-op
	var m = _mech(victim)
	if m == null or m.dead:
		return
	var armor: float = maxf(m.armor - dmg, 0.0)
	_armor.rpc(victim, armor, shooter)


@rpc("authority", "call_local", "reliable")
func _armor(victim: String, armor: float, shooter: String) -> void:
	var m = _mech(victim)
	if m == null:
		return
	m.set_armor(armor)
	if armor <= 0.0 and not m.dead:
		m.die()
		_score(shooter, "kills")
		_score(victim, "deaths")
		game_event.emit("%s destroyed %s" % [_name(shooter), _name(victim)])


func target_hit(index: int) -> void:
	if not multiplayer.is_server() or game == null or index < 0 or index >= game.targets.size():
		return
	var t = game.targets[index]
	if t.hp <= 0:
		return
	_target_hp.rpc(index, t.hp - 1)


@rpc("authority", "call_local", "reliable")
func _target_hp(index: int, hp: int) -> void:
	if game == null or index >= game.targets.size():
		return
	var t = game.targets[index]
	var was_alive: bool = t.hp > 0
	t.apply_hp(hp)
	if was_alive and hp <= 0:
		team_score += 1
		game_event.emit("Turret destroyed")


@rpc("authority", "reliable")
func _target_states(hps: Array) -> void:
	if game == null:
		return
	for i in mini(hps.size(), game.targets.size()):
		game.targets[i].apply_hp(int(hps[i]))


func _score(id: String, key: String) -> void:
	if not scores.has(id):
		scores[id] = {"kills": 0, "deaths": 0}
	scores[id][key] += 1


func _name(id: String) -> String:
	for e in roster:
		if e.id == id:
			return e.name
	return "?"


func _mech(id: String):
	if game == null:
		return null
	return game.mechs.get(id)


func _status(t: String) -> void:
	status_changed.emit(t)


# ---------------------------------------------------------------- LAN discovery
# Clients broadcast "QUADMECH?" to the discovery port (and to localhost, so seats on
# the same machine find each other); the host answers with its name and mode.
# Clients use a random local port, so any number can browse at once.

func start_browsing() -> void:
	_stop_udp()
	_udp = PacketPeerUDP.new()
	_udp.set_broadcast_enabled(true)
	if _udp.bind(0) != OK:
		_status("Can't search the network")
		return
	_browsing = true
	_browse_t = 0.0
	found_hosts.clear()


func stop_browsing() -> void:
	if _browsing:
		_browsing = false
		_stop_udp()


func _start_responder() -> void:
	_stop_udp()
	_udp = PacketPeerUDP.new()
	if _udp.bind(DISCOVERY_PORT) != OK:
		_status("Hosting (LAN discovery unavailable: port %d busy)" % DISCOVERY_PORT)
		_udp = null
		return
	_responding = true


func _stop_udp() -> void:
	if _udp != null:
		_udp.close()
		_udp = null


func _process(delta: float) -> void:
	if _udp == null:
		return
	if _browsing:
		_browse_t -= delta
		if _browse_t <= 0.0:
			_browse_t = 1.0
			for addr in ["255.255.255.255", "127.0.0.1"]:
				_udp.set_dest_address(addr, DISCOVERY_PORT)
				_udp.put_packet("QUADMECH?".to_utf8_buffer())
			var now := Time.get_ticks_msec()
			var changed := false
			for ip in found_hosts.keys():
				if now - int(found_hosts[ip].time) > 4000:
					found_hosts.erase(ip)
					changed = true
			if changed:
				hosts_changed.emit()
	while _udp != null and _udp.get_available_packet_count() > 0:
		var pkt := _udp.get_packet().get_string_from_utf8()
		var ip := _udp.get_packet_ip()
		var port := _udp.get_packet_port()
		if _responding and pkt == "QUADMECH?":
			var info := "QUADMECH!|%s|%s|%d" % [_host_name, mode, roster.size()]
			_udp.set_dest_address(ip, port)
			_udp.put_packet(info.to_utf8_buffer())
		elif _browsing and pkt.begins_with("QUADMECH!|"):
			var parts := pkt.split("|")
			if parts.size() >= 4:
				# A host on this same machine answers on both its LAN address and
				# localhost; keep only the localhost entry for it.
				var dup := false
				for other in found_hosts.keys():
					if other != ip and found_hosts[other].name == parts[1]:
						if ip == "127.0.0.1":
							found_hosts.erase(other)
						elif other == "127.0.0.1":
							dup = true
				if not dup:
					found_hosts[ip] = {"name": parts[1], "mode": parts[2], "players": int(parts[3]), "time": Time.get_ticks_msec()}
					hosts_changed.emit()
