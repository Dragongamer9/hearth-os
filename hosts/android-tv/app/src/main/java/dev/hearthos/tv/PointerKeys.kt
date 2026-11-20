package dev.hearthos.tv

import android.view.KeyEvent

/**
 * The remote's buttons on a website in Theater's viewer (ViewerActivity), and the pointer's settings. Pure, so the
 * unit tests (src/test) cover them without a TV.
 */
enum class PointerKey(val js: String = "") {
    UP("up"), DOWN("down"), LEFT("left"), RIGHT("right"), OK, BACK, MODE,
    SCROLL_UP, SCROLL_DOWN, SCROLL_LEFT, SCROLL_RIGHT, PLAYPAUSE, FF, REW, FULLSCREEN, OTHER;

    val isDirection get() = js.isNotEmpty()

    companion object {
        /** Holding OK this long in page mode brings the pointer back (pointer.js uses the same time for its menu). */
        const val LONG_MS = 600L

        fun of(code: Int): PointerKey = when (code) {
            KeyEvent.KEYCODE_DPAD_UP -> UP
            KeyEvent.KEYCODE_DPAD_DOWN -> DOWN
            KeyEvent.KEYCODE_DPAD_LEFT -> LEFT
            KeyEvent.KEYCODE_DPAD_RIGHT -> RIGHT
            KeyEvent.KEYCODE_DPAD_CENTER, KeyEvent.KEYCODE_ENTER, KeyEvent.KEYCODE_NUMPAD_ENTER, KeyEvent.KEYCODE_BUTTON_A -> OK
            KeyEvent.KEYCODE_BACK, KeyEvent.KEYCODE_BUTTON_B -> BACK
            // Switch between pointer and page mode: Menu, a controller's Start, or a colour key when the remote has them.
            KeyEvent.KEYCODE_MENU, KeyEvent.KEYCODE_BUTTON_START, KeyEvent.KEYCODE_PROG_RED, KeyEvent.KEYCODE_PROG_GREEN,
            KeyEvent.KEYCODE_PROG_YELLOW, KeyEvent.KEYCODE_PROG_BLUE -> MODE
            // Scroll keys: channel up/down, page up/down, the number keys 2 and 8 as on a remote's keypad (4 and 6 sideways).
            KeyEvent.KEYCODE_CHANNEL_UP, KeyEvent.KEYCODE_PAGE_UP, KeyEvent.KEYCODE_2 -> SCROLL_UP
            KeyEvent.KEYCODE_CHANNEL_DOWN, KeyEvent.KEYCODE_PAGE_DOWN, KeyEvent.KEYCODE_8 -> SCROLL_DOWN
            KeyEvent.KEYCODE_4, KeyEvent.KEYCODE_NUMPAD_4 -> SCROLL_LEFT
            KeyEvent.KEYCODE_6, KeyEvent.KEYCODE_NUMPAD_6 -> SCROLL_RIGHT
            KeyEvent.KEYCODE_MEDIA_PLAY_PAUSE, KeyEvent.KEYCODE_MEDIA_PLAY, KeyEvent.KEYCODE_MEDIA_PAUSE, KeyEvent.KEYCODE_BUTTON_X -> PLAYPAUSE
            KeyEvent.KEYCODE_MEDIA_FAST_FORWARD -> FF
            KeyEvent.KEYCODE_MEDIA_REWIND -> REW
            KeyEvent.KEYCODE_BUTTON_Y -> FULLSCREEN
            else -> OTHER
        }

        /** A held key (from its first press to this repeat) counts as a long press. */
        fun isLong(downTime: Long, eventTime: Long) = eventTime - downTime >= LONG_MS

        /** A wheel's turn for [px] pixels of scrolling: Android's vertical wheel turns up for positive values. */
        fun wheelV(px: Float, factor: Float): Float = if (factor <= 0f) 0f else -px / factor
        fun wheelH(px: Float, factor: Float): Float = if (factor <= 0f) 0f else px / factor
    }
}

/** Settings › Remote and controllers › Pointer on websites (settings.json "pointer"; defaults here are never written). */
data class PointerSettings(val speed: String, val size: String, val startOn: Boolean) {
    companion object {
        val SPEEDS = setOf("slow", "normal", "fast")
        val SIZES = setOf("s", "m", "l")
        fun of(speed: String?, size: String?, startOn: Boolean?) = PointerSettings(
            if (speed in SPEEDS) speed!! else "normal",
            if (size in SIZES) size!! else "m",
            startOn ?: true
        )
    }
}
