package dev.hearthos.android

import android.view.KeyEvent

/**
 * Controllers on Android: their buttons arrive as KeyEvents and their sticks as joystick MotionEvents. Hearth
 * reads them here and hands the page the button names its own controller code uses ("pad" events, the same as the
 * Linux and Windows hosts send), so it works the same on every controller, whatever WebView's Gamepad API does.
 */
object Pad {
    /** The page's name for a controller button (null: not a controller button Hearth uses). */
    fun buttonName(keyCode: Int): String? = when (keyCode) {
        KeyEvent.KEYCODE_BUTTON_A, KeyEvent.KEYCODE_DPAD_CENTER -> "a"
        KeyEvent.KEYCODE_BUTTON_B -> "b"
        KeyEvent.KEYCODE_BUTTON_X -> "x"
        KeyEvent.KEYCODE_BUTTON_Y -> "y"
        KeyEvent.KEYCODE_BUTTON_L1 -> "lb"
        KeyEvent.KEYCODE_BUTTON_R1 -> "rb"
        KeyEvent.KEYCODE_BUTTON_START, KeyEvent.KEYCODE_MENU -> "start"
        KeyEvent.KEYCODE_BUTTON_SELECT -> "select"
        KeyEvent.KEYCODE_BUTTON_MODE -> "guide"
        KeyEvent.KEYCODE_DPAD_UP -> "up"
        KeyEvent.KEYCODE_DPAD_DOWN -> "down"
        KeyEvent.KEYCODE_DPAD_LEFT -> "left"
        KeyEvent.KEYCODE_DPAD_RIGHT -> "right"
        KeyEvent.KEYCODE_ENTER -> "a"
        else -> null
    }

    val DIRECTIONS = setOf("up", "down", "left", "right")

    /**
     * Which directions a stick position holds. A held direction lets go only nearer the middle (0.35) than
     * where it started (0.55), so a stick resting near the edge doesn't flicker. The hat (some controllers' d-pad)
     * counts at 0.5; the right stick's up/down jumps through the library A to Z ("rup"/"rdown").
     */
    fun directions(x: Float, y: Float, hatX: Float, hatY: Float, rightY: Float, held: Set<String>): Set<String> {
        fun th(k: String) = if (k in held) 0.35f else 0.55f
        val out = HashSet<String>()
        if (y < -th("up") || hatY < -0.5f) out.add("up")
        if (y > th("down") || hatY > 0.5f) out.add("down")
        if (x < -th("left") || hatX < -0.5f) out.add("left")
        if (x > th("right") || hatX > 0.5f) out.add("right")
        if (rightY < -0.6f) out.add("rup")
        if (rightY > 0.6f) out.add("rdown")
        return out
    }
}

/**
 * Key repeat for held directions (stick or d-pad): the first press at once, then after 400 ms every 110 ms, the
 * page's own rhythm (pollPads in ui/index.html).
 */
class Repeater(private val delayMs: Long = 400, private val everyMs: Long = 110) {
    private class Held(val t0: Long, var last: Long)
    private val held = HashMap<String, Held>()

    val holding: Set<String> get() = held.keys

    /** The new state of the held directions: returns the ones newly pressed. */
    fun update(now: Set<String>, t: Long): List<String> {
        val pressed = now.filter { it !in held }
        held.keys.retainAll(now)
        for (k in pressed) held[k] = Held(t, t)
        return pressed
    }

    /** Called often while something is held: returns the repeats due (only up/down/left/right repeat). */
    fun tick(t: Long): List<String> = held.filter { (k, h) -> k in Pad.DIRECTIONS && t - h.t0 > delayMs && t - h.last >= everyMs }
        .map { (k, h) -> h.last = t; k }

    fun clear() = held.clear()
}
