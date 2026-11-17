.pragma library
// Search plugin: commands (like KRunner's shell runner). ">htop" or "> ls -la" runs it: in the terminal (Enter, so you
// see what it prints) or on its own (the second row, for programs with their own window). Nothing runs until you pick a row.

function command(qRaw) {
    var q = String(qRaw || "");
    var m = /^\s*>\s*(.+?)\s*$/.exec(q);
    return m ? m[1] : null;
}

function results(q) {
    var c = command(q);
    if (!c) return [];
    return [
        { plugin: "commands", group: "Run", title: "Run “" + c + "” in the terminal", sub: "Opens Terminal and runs it there", icon: "kbd",
          score: 2100, top: true, action: { type: "command", value: c, terminal: true } },
        { plugin: "commands", group: "Run", title: "Run “" + c + "” on its own", sub: "For programs with their own window", icon: "kbd",
          score: 2090, action: { type: "command", value: c, terminal: false } }
    ];
}
