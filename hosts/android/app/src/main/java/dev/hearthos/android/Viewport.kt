package dev.hearthos.android

/**
 * How wide Hearth's page is laid out on this screen (in CSS pixels; 0 = the screen's own width), from the screen's
 * size in dp. Hearth's page is designed for a Steam Deck-like 1280×800, so:
 *
 *  - Landscape narrower than 1280 dp (phones, the Galaxy Tab A9 8.7" at about 1005×600 dp, most handhelds): laid out
 *    1280 px wide and scaled to fit, the size it's made for (the Tab A9's 1340×800 screen then looks like a Deck's).
 *  - Portrait narrower than 800 dp (tablets held upright, like the Tab A9 at about 600×1005 dp): laid out 800 px wide,
 *    where the page's own narrow-screen layout takes over.
 *  - Anything bigger: the screen's own width.
 *
 * tools/android-ui-check.js lays the page out the same way for its pictures (keep the two in step).
 */
object Viewport {
    const val LANDSCAPE_WIDTH = 1280
    const val PORTRAIT_WIDTH = 800
    /** Tablets (smallest width 600 dp and up) may turn to portrait; phones stay landscape (the page's controller
     *  layout doesn't fit a phone held upright). */
    const val TABLET_SW_DP = 600

    fun width(widthDp: Int, heightDp: Int): Int = when {
        widthDp <= 0 || heightDp <= 0 -> LANDSCAPE_WIDTH
        widthDp >= heightDp -> if (widthDp < LANDSCAPE_WIDTH) LANDSCAPE_WIDTH else 0
        else -> if (widthDp < PORTRAIT_WIDTH) PORTRAIT_WIDTH else 0
    }

    fun isTablet(smallestWidthDp: Int): Boolean = smallestWidthDp >= TABLET_SW_DP

    /** The page's viewport meta content for a screen [widthDp] wide laid out [width] px wide (from [width]). The
     *  scale is pinned (minimum = maximum) so a turn of the screen, which changes this without reloading the page,
     *  always fits the page to the new width. */
    fun meta(width: Int, widthDp: Int): String {
        if (width <= 0 || widthDp <= 0) return "width=device-width, initial-scale=1, viewport-fit=cover"
        val s = String.format(java.util.Locale.US, "%.4f", widthDp.toDouble() / width)
        return "width=$width, initial-scale=$s, minimum-scale=$s, maximum-scale=$s, user-scalable=no, viewport-fit=cover"
    }
}
