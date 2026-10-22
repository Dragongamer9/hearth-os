// Hearth Browser: Firefox reads hearth.cfg (next to the firefox program) at every start. It adds Hearth's look,
// the on-screen keyboard and the start page (hearth/hearth.sys.mjs). Firefox's own AutoConfig, which release
// builds support, so Mozilla's unchanged Firefox can be used.
pref("general.config.filename", "hearth.cfg");
pref("general.config.obscure_value", 0);
pref("general.config.sandbox_enabled", false);
