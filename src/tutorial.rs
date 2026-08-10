pub struct TutorialStep {
    pub title: &'static str,
    pub lines: &'static [&'static str],
}

pub const STEPS: &[TutorialStep] = &[
    TutorialStep {
        title: "welcome to confy!",
        lines: &[
            "confy tracks your config files in one place.",
            "use j/k (or arrow keys) to move up and down.",
            "press enter to open a file in your $EDITOR.",
            "press p to toggle a live preview pane.",
            "",
            "press any key for the next tip...",
        ],
    },
    TutorialStep {
        title: "groups",
        lines: &[
            "files are organised into groups.",
            "press enter or space on a group to collapse/expand it.",
            "",
            ":ag <name>  →  add a group",
            ":rg <name>  →  remove a group (files go to ungrouped)",
            ":mg <name>  →  move selected file to a group",
            "",
            "press any key for the next tip...",
        ],
    },
    TutorialStep {
        title: "commands  (press : to enter)",
        lines: &[
            ":ac          →  add a config file (opens file picker)",
            ":ac <group>  →  add directly to a group",
            ":rm          →  remove selected file from tracking",
            ":l           →  reopen last edited file",
            ":cd          →  change the config search directory",
            ":sort name|date|size  →  sort files",
            ":reverse     →  flip sort order",
            ":theme <name>  →  switch color theme",
            ":device <host>  →  browse a remote host's configs",
            ":device local  →  back to local configs",
            ":su  →  edit selected file as root (needs polkit)",
            ":h / :help   →  show this tutorial again",
            "",
            "press any key for the next tip...",
        ],
    },
    TutorialStep {
        title: "search  (press / to enter)",
        lines: &[
            "type to filter files and groups live.",
            "press enter to confirm, esc to clear.",
            "",
            "rollback  (:rb)",
            "confy saves a backup to /tmp whenever you",
            "open a file for editing. :rb restores it.",
            "set  \"rollback\": false  in config.json to disable.",
            "",
            "press any key for the next tip...",
        ],
    },
    TutorialStep {
        title: "that's it!",
        lines: &[
            "tip: set your $EDITOR env var to your preferred",
            "editor (vim, nvim, nano, micro, etc.).",
            "",
            "for full docs run:  man confy",
            "(or just poke around, there isn't much to break!)",
            "",
            "press any key to start...",
        ],
    },
];
