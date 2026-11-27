pub struct IconDef {
    pub rust_name: &'static str,
    pub tabler_name: &'static str,
    pub view_box: &'static str,
    pub raw_paths: &'static [&'static str],
    pub d_values: &'static [&'static str],
}

pub const ICON_DEFS: &[IconDef] = &[
    IconDef {
        rust_name: "IconUser",
        tabler_name: "user",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M8 7a4 4 0 1 0 8 0a4 4 0 0 0 -8 0\" />", "<path d=\"M6 21v-2a4 4 0 0 1 4 -4h4a4 4 0 0 1 4 4v2\" />"],
        d_values: &["M0 0h24v24H0z", "M8 7a4 4 0 1 0 8 0a4 4 0 0 0 -8 0", "M6 21v-2a4 4 0 0 1 4 -4h4a4 4 0 0 1 4 4v2"],
    },
    IconDef {
        rust_name: "IconUsers",
        tabler_name: "users",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M5 7a4 4 0 1 0 8 0a4 4 0 1 0 -8 0\" />", "<path d=\"M3 21v-2a4 4 0 0 1 4 -4h4a4 4 0 0 1 4 4v2\" />", "<path d=\"M16 3.13a4 4 0 0 1 0 7.75\" />", "<path d=\"M21 21v-2a4 4 0 0 0 -3 -3.85\" />"],
        d_values: &["M0 0h24v24H0z", "M5 7a4 4 0 1 0 8 0a4 4 0 1 0 -8 0", "M3 21v-2a4 4 0 0 1 4 -4h4a4 4 0 0 1 4 4v2", "M16 3.13a4 4 0 0 1 0 7.75", "M21 21v-2a4 4 0 0 0 -3 -3.85"],
    },
    IconDef {
        rust_name: "IconMail",
        tabler_name: "mail",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M3 7a2 2 0 0 1 2 -2h14a2 2 0 0 1 2 2v10a2 2 0 0 1 -2 2h-14a2 2 0 0 1 -2 -2v-10\" />", "<path d=\"M3 7l9 6l9 -6\" />"],
        d_values: &["M0 0h24v24H0z", "M3 7a2 2 0 0 1 2 -2h14a2 2 0 0 1 2 2v10a2 2 0 0 1 -2 2h-14a2 2 0 0 1 -2 -2v-10", "M3 7l9 6l9 -6"],
    },
    IconDef {
        rust_name: "IconLock",
        tabler_name: "lock",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M5 13a2 2 0 0 1 2 -2h10a2 2 0 0 1 2 2v6a2 2 0 0 1 -2 2h-10a2 2 0 0 1 -2 -2v-6\" />", "<path d=\"M11 16a1 1 0 1 0 2 0a1 1 0 0 0 -2 0\" />", "<path d=\"M8 11v-4a4 4 0 1 1 8 0v4\" />"],
        d_values: &["M0 0h24v24H0z", "M5 13a2 2 0 0 1 2 -2h10a2 2 0 0 1 2 2v6a2 2 0 0 1 -2 2h-10a2 2 0 0 1 -2 -2v-6", "M11 16a1 1 0 1 0 2 0a1 1 0 0 0 -2 0", "M8 11v-4a4 4 0 1 1 8 0v4"],
    },
    IconDef {
        rust_name: "IconEye",
        tabler_name: "eye",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M10 12a2 2 0 1 0 4 0a2 2 0 0 0 -4 0\" />", "<path d=\"M21 12c-2.4 4 -5.4 6 -9 6c-3.6 0 -6.6 -2 -9 -6c2.4 -4 5.4 -6 9 -6c3.6 0 6.6 2 9 6\" />"],
        d_values: &["M0 0h24v24H0z", "M10 12a2 2 0 1 0 4 0a2 2 0 0 0 -4 0", "M21 12c-2.4 4 -5.4 6 -9 6c-3.6 0 -6.6 -2 -9 -6c2.4 -4 5.4 -6 9 -6c3.6 0 6.6 2 9 6"],
    },
    IconDef {
        rust_name: "IconEyeOff",
        tabler_name: "eye-off",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M10.585 10.587a2 2 0 0 0 2.829 2.828\" />", "<path d=\"M16.681 16.673a8.717 8.717 0 0 1 -4.681 1.327c-3.6 0 -6.6 -2 -9 -6c1.272 -2.12 2.712 -3.678 4.32 -4.674m2.86 -1.146a9.055 9.055 0 0 1 1.82 -.18c3.6 0 6.6 2 9 6c-.666 1.11 -1.379 2.067 -2.138 2.87\" />", "<path d=\"M3 3l18 18\" />"],
        d_values: &["M0 0h24v24H0z", "M10.585 10.587a2 2 0 0 0 2.829 2.828", "M16.681 16.673a8.717 8.717 0 0 1 -4.681 1.327c-3.6 0 -6.6 -2 -9 -6c1.272 -2.12 2.712 -3.678 4.32 -4.674m2.86 -1.146a9.055 9.055 0 0 1 1.82 -.18c3.6 0 6.6 2 9 6c-.666 1.11 -1.379 2.067 -2.138 2.87", "M3 3l18 18"],
    },
    IconDef {
        rust_name: "IconCheck",
        tabler_name: "check",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M5 12l5 5l10 -10\" />"],
        d_values: &["M0 0h24v24H0z", "M5 12l5 5l10 -10"],
    },
    IconDef {
        rust_name: "IconX",
        tabler_name: "x",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M18 6l-12 12\" />", "<path d=\"M6 6l12 12\" />"],
        d_values: &["M0 0h24v24H0z", "M18 6l-12 12", "M6 6l12 12"],
    },
    IconDef {
        rust_name: "IconChevronDown",
        tabler_name: "chevron-down",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M6 9l6 6l6 -6\" />"],
        d_values: &["M0 0h24v24H0z", "M6 9l6 6l6 -6"],
    },
    IconDef {
        rust_name: "IconChevronUp",
        tabler_name: "chevron-up",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M6 15l6 -6l6 6\" />"],
        d_values: &["M0 0h24v24H0z", "M6 15l6 -6l6 6"],
    },
    IconDef {
        rust_name: "IconChevronLeft",
        tabler_name: "chevron-left",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M15 6l-6 6l6 6\" />"],
        d_values: &["M0 0h24v24H0z", "M15 6l-6 6l6 6"],
    },
    IconDef {
        rust_name: "IconChevronRight",
        tabler_name: "chevron-right",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M9 6l6 6l-6 6\" />"],
        d_values: &["M0 0h24v24H0z", "M9 6l6 6l-6 6"],
    },
    IconDef {
        rust_name: "IconArrowRight",
        tabler_name: "arrow-right",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M5 12l14 0\" />", "<path d=\"M13 18l6 -6\" />", "<path d=\"M13 6l6 6\" />"],
        d_values: &["M0 0h24v24H0z", "M5 12l14 0", "M13 18l6 -6", "M13 6l6 6"],
    },
    IconDef {
        rust_name: "IconSun",
        tabler_name: "sun",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M8 12a4 4 0 1 0 8 0a4 4 0 1 0 -8 0\" />", "<path d=\"M3 12h1m8 -9v1m8 8h1m-9 8v1m-6.4 -15.4l.7 .7m12.1 -.7l-.7 .7m0 11.4l.7 .7m-12.1 -.7l-.7 .7\" />"],
        d_values: &["M0 0h24v24H0z", "M8 12a4 4 0 1 0 8 0a4 4 0 1 0 -8 0", "M3 12h1m8 -9v1m8 8h1m-9 8v1m-6.4 -15.4l.7 .7m12.1 -.7l-.7 .7m0 11.4l.7 .7m-12.1 -.7l-.7 .7"],
    },
    IconDef {
        rust_name: "IconMoon",
        tabler_name: "moon",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M12 3c.132 0 .263 0 .393 0a7.5 7.5 0 0 0 7.92 12.446a9 9 0 1 1 -8.313 -12.454l0 .008\" />"],
        d_values: &["M0 0h24v24H0z", "M12 3c.132 0 .263 0 .393 0a7.5 7.5 0 0 0 7.92 12.446a9 9 0 1 1 -8.313 -12.454l0 .008"],
    },
    IconDef {
        rust_name: "IconSettings",
        tabler_name: "settings",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M10.325 4.317c.426 -1.756 2.924 -1.756 3.35 0a1.724 1.724 0 0 0 2.573 1.066c1.543 -.94 3.31 .826 2.37 2.37a1.724 1.724 0 0 0 1.065 2.572c1.756 .426 1.756 2.924 0 3.35a1.724 1.724 0 0 0 -1.066 2.573c.94 1.543 -.826 3.31 -2.37 2.37a1.724 1.724 0 0 0 -2.572 1.065c-.426 1.756 -2.924 1.756 -3.35 0a1.724 1.724 0 0 0 -2.573 -1.066c-1.543 .94 -3.31 -.826 -2.37 -2.37a1.724 1.724 0 0 0 -1.065 -2.572c-1.756 -.426 -1.756 -2.924 0 -3.35a1.724 1.724 0 0 0 1.066 -2.573c-.94 -1.543 .826 -3.31 2.37 -2.37c1 .608 2.296 .07 2.572 -1.065\" />", "<path d=\"M9 12a3 3 0 1 0 6 0a3 3 0 0 0 -6 0\" />"],
        d_values: &["M0 0h24v24H0z", "M10.325 4.317c.426 -1.756 2.924 -1.756 3.35 0a1.724 1.724 0 0 0 2.573 1.066c1.543 -.94 3.31 .826 2.37 2.37a1.724 1.724 0 0 0 1.065 2.572c1.756 .426 1.756 2.924 0 3.35a1.724 1.724 0 0 0 -1.066 2.573c.94 1.543 -.826 3.31 -2.37 2.37a1.724 1.724 0 0 0 -2.572 1.065c-.426 1.756 -2.924 1.756 -3.35 0a1.724 1.724 0 0 0 -2.573 -1.066c-1.543 .94 -3.31 -.826 -2.37 -2.37a1.724 1.724 0 0 0 -1.065 -2.572c-1.756 -.426 -1.756 -2.924 0 -3.35a1.724 1.724 0 0 0 1.066 -2.573c-.94 -1.543 .826 -3.31 2.37 -2.37c1 .608 2.296 .07 2.572 -1.065", "M9 12a3 3 0 1 0 6 0a3 3 0 0 0 -6 0"],
    },
    IconDef {
        rust_name: "IconBuilding",
        tabler_name: "building",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M3 21l18 0\" />", "<path d=\"M9 8l1 0\" />", "<path d=\"M9 12l1 0\" />", "<path d=\"M9 16l1 0\" />", "<path d=\"M14 8l1 0\" />", "<path d=\"M14 12l1 0\" />", "<path d=\"M14 16l1 0\" />", "<path d=\"M5 21v-16a2 2 0 0 1 2 -2h10a2 2 0 0 1 2 2v16\" />"],
        d_values: &["M0 0h24v24H0z", "M3 21l18 0", "M9 8l1 0", "M9 12l1 0", "M9 16l1 0", "M14 8l1 0", "M14 12l1 0", "M14 16l1 0", "M5 21v-16a2 2 0 0 1 2 -2h10a2 2 0 0 1 2 2v16"],
    },
    IconDef {
        rust_name: "IconBuildingSkyscraper",
        tabler_name: "building-skyscraper",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M3 21l18 0\" />", "<path d=\"M5 21v-14l8 -4v18\" />", "<path d=\"M19 21v-10l-6 -4\" />", "<path d=\"M9 9l0 .01\" />", "<path d=\"M9 12l0 .01\" />", "<path d=\"M9 15l0 .01\" />", "<path d=\"M9 18l0 .01\" />"],
        d_values: &["M0 0h24v24H0z", "M3 21l18 0", "M5 21v-14l8 -4v18", "M19 21v-10l-6 -4", "M9 9l0 .01", "M9 12l0 .01", "M9 15l0 .01", "M9 18l0 .01"],
    },
    IconDef {
        rust_name: "IconHome",
        tabler_name: "home",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M5 12l-2 0l9 -9l9 9l-2 0\" />", "<path d=\"M5 12v7a2 2 0 0 0 2 2h10a2 2 0 0 0 2 -2v-7\" />", "<path d=\"M9 21v-6a2 2 0 0 1 2 -2h2a2 2 0 0 1 2 2v6\" />"],
        d_values: &["M0 0h24v24H0z", "M5 12l-2 0l9 -9l9 9l-2 0", "M5 12v7a2 2 0 0 0 2 2h10a2 2 0 0 0 2 -2v-7", "M9 21v-6a2 2 0 0 1 2 -2h2a2 2 0 0 1 2 2v6"],
    },
    IconDef {
        rust_name: "IconSearch",
        tabler_name: "search",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M3 10a7 7 0 1 0 14 0a7 7 0 1 0 -14 0\" />", "<path d=\"M21 21l-6 -6\" />"],
        d_values: &["M0 0h24v24H0z", "M3 10a7 7 0 1 0 14 0a7 7 0 1 0 -14 0", "M21 21l-6 -6"],
    },
    IconDef {
        rust_name: "IconPlus",
        tabler_name: "plus",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M12 5l0 14\" />", "<path d=\"M5 12l14 0\" />"],
        d_values: &["M0 0h24v24H0z", "M12 5l0 14", "M5 12l14 0"],
    },
    IconDef {
        rust_name: "IconSparkles",
        tabler_name: "sparkles",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M16 18a2 2 0 0 1 2 2a2 2 0 0 1 2 -2a2 2 0 0 1 -2 -2a2 2 0 0 1 -2 2m0 -12a2 2 0 0 1 2 2a2 2 0 0 1 2 -2a2 2 0 0 1 -2 -2a2 2 0 0 1 -2 2m-7 12a6 6 0 0 1 6 -6a6 6 0 0 1 -6 -6a6 6 0 0 1 -6 6a6 6 0 0 1 6 6\" />"],
        d_values: &["M0 0h24v24H0z", "M16 18a2 2 0 0 1 2 2a2 2 0 0 1 2 -2a2 2 0 0 1 -2 -2a2 2 0 0 1 -2 2m0 -12a2 2 0 0 1 2 2a2 2 0 0 1 2 -2a2 2 0 0 1 -2 -2a2 2 0 0 1 -2 2m-7 12a6 6 0 0 1 6 -6a6 6 0 0 1 -6 -6a6 6 0 0 1 -6 6a6 6 0 0 1 6 6"],
    },
    IconDef {
        rust_name: "IconTable",
        tabler_name: "table",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M3 5a2 2 0 0 1 2 -2h14a2 2 0 0 1 2 2v14a2 2 0 0 1 -2 2h-14a2 2 0 0 1 -2 -2v-14\" />", "<path d=\"M3 10h18\" />", "<path d=\"M10 3v18\" />"],
        d_values: &["M0 0h24v24H0z", "M3 5a2 2 0 0 1 2 -2h14a2 2 0 0 1 2 2v14a2 2 0 0 1 -2 2h-14a2 2 0 0 1 -2 -2v-14", "M3 10h18", "M10 3v18"],
    },
    IconDef {
        rust_name: "IconLayoutList",
        tabler_name: "layout-list",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M4 6a2 2 0 0 1 2 -2h12a2 2 0 0 1 2 2v2a2 2 0 0 1 -2 2h-12a2 2 0 0 1 -2 -2l0 -2\" />", "<path d=\"M4 16a2 2 0 0 1 2 -2h12a2 2 0 0 1 2 2v2a2 2 0 0 1 -2 2h-12a2 2 0 0 1 -2 -2l0 -2\" />"],
        d_values: &["M0 0h24v24H0z", "M4 6a2 2 0 0 1 2 -2h12a2 2 0 0 1 2 2v2a2 2 0 0 1 -2 2h-12a2 2 0 0 1 -2 -2l0 -2", "M4 16a2 2 0 0 1 2 -2h12a2 2 0 0 1 2 2v2a2 2 0 0 1 -2 2h-12a2 2 0 0 1 -2 -2l0 -2"],
    },
    IconDef {
        rust_name: "IconLayoutKanban",
        tabler_name: "layout-kanban",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M4 4l6 0\" />", "<path d=\"M14 4l6 0\" />", "<path d=\"M4 10a2 2 0 0 1 2 -2h2a2 2 0 0 1 2 2v8a2 2 0 0 1 -2 2h-2a2 2 0 0 1 -2 -2l0 -8\" />", "<path d=\"M14 10a2 2 0 0 1 2 -2h2a2 2 0 0 1 2 2v2a2 2 0 0 1 -2 2h-2a2 2 0 0 1 -2 -2l0 -2\" />"],
        d_values: &["M0 0h24v24H0z", "M4 4l6 0", "M14 4l6 0", "M4 10a2 2 0 0 1 2 -2h2a2 2 0 0 1 2 2v8a2 2 0 0 1 -2 2h-2a2 2 0 0 1 -2 -2l0 -8", "M14 10a2 2 0 0 1 2 -2h2a2 2 0 0 1 2 2v2a2 2 0 0 1 -2 2h-2a2 2 0 0 1 -2 -2l0 -2"],
    },
    IconDef {
        rust_name: "IconCalendar",
        tabler_name: "calendar",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M4 7a2 2 0 0 1 2 -2h12a2 2 0 0 1 2 2v12a2 2 0 0 1 -2 2h-12a2 2 0 0 1 -2 -2v-12\" />", "<path d=\"M16 3v4\" />", "<path d=\"M8 3v4\" />", "<path d=\"M4 11h16\" />", "<path d=\"M11 15h1\" />", "<path d=\"M12 15v3\" />"],
        d_values: &["M0 0h24v24H0z", "M4 7a2 2 0 0 1 2 -2h12a2 2 0 0 1 2 2v12a2 2 0 0 1 -2 2h-12a2 2 0 0 1 -2 -2v-12", "M16 3v4", "M8 3v4", "M4 11h16", "M11 15h1", "M12 15v3"],
    },
    IconDef {
        rust_name: "IconFilter",
        tabler_name: "filter",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M4 4h16v2.172a2 2 0 0 1 -.586 1.414l-4.414 4.414v7l-6 2v-8.5l-4.48 -4.928a2 2 0 0 1 -.52 -1.345v-2.227\" />"],
        d_values: &["M0 0h24v24H0z", "M4 4h16v2.172a2 2 0 0 1 -.586 1.414l-4.414 4.414v7l-6 2v-8.5l-4.48 -4.928a2 2 0 0 1 -.52 -1.345v-2.227"],
    },
    IconDef {
        rust_name: "IconArrowsSort",
        tabler_name: "arrows-sort",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M3 9l4 -4l4 4m-4 -4v14\" />", "<path d=\"M21 15l-4 4l-4 -4m4 4v-14\" />"],
        d_values: &["M0 0h24v24H0z", "M3 9l4 -4l4 4m-4 -4v14", "M21 15l-4 4l-4 -4m4 4v-14"],
    },
    IconDef {
        rust_name: "IconSelector",
        tabler_name: "selector",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M8 9l4 -4l4 4\" />", "<path d=\"M16 15l-4 4l-4 -4\" />"],
        d_values: &["M0 0h24v24H0z", "M8 9l4 -4l4 4", "M16 15l-4 4l-4 -4"],
    },
    IconDef {
        rust_name: "IconDots",
        tabler_name: "dots",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M4 12a1 1 0 1 0 2 0a1 1 0 1 0 -2 0\" />", "<path d=\"M11 12a1 1 0 1 0 2 0a1 1 0 1 0 -2 0\" />", "<path d=\"M18 12a1 1 0 1 0 2 0a1 1 0 1 0 -2 0\" />"],
        d_values: &["M0 0h24v24H0z", "M4 12a1 1 0 1 0 2 0a1 1 0 1 0 -2 0", "M11 12a1 1 0 1 0 2 0a1 1 0 1 0 -2 0", "M18 12a1 1 0 1 0 2 0a1 1 0 1 0 -2 0"],
    },
    IconDef {
        rust_name: "IconDotsVertical",
        tabler_name: "dots-vertical",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M11 12a1 1 0 1 0 2 0a1 1 0 1 0 -2 0\" />", "<path d=\"M11 19a1 1 0 1 0 2 0a1 1 0 1 0 -2 0\" />", "<path d=\"M11 5a1 1 0 1 0 2 0a1 1 0 1 0 -2 0\" />"],
        d_values: &["M0 0h24v24H0z", "M11 12a1 1 0 1 0 2 0a1 1 0 1 0 -2 0", "M11 19a1 1 0 1 0 2 0a1 1 0 1 0 -2 0", "M11 5a1 1 0 1 0 2 0a1 1 0 1 0 -2 0"],
    },
    IconDef {
        rust_name: "IconPencil",
        tabler_name: "pencil",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M4 20h4l10.5 -10.5a2.828 2.828 0 1 0 -4 -4l-10.5 10.5v4\" />", "<path d=\"M13.5 6.5l4 4\" />"],
        d_values: &["M0 0h24v24H0z", "M4 20h4l10.5 -10.5a2.828 2.828 0 1 0 -4 -4l-10.5 10.5v4", "M13.5 6.5l4 4"],
    },
    IconDef {
        rust_name: "IconTrash",
        tabler_name: "trash",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M4 7l16 0\" />", "<path d=\"M10 11l0 6\" />", "<path d=\"M14 11l0 6\" />", "<path d=\"M5 7l1 12a2 2 0 0 0 2 2h8a2 2 0 0 0 2 -2l1 -12\" />", "<path d=\"M9 7v-3a1 1 0 0 1 1 -1h4a1 1 0 0 1 1 1v3\" />"],
        d_values: &["M0 0h24v24H0z", "M4 7l16 0", "M10 11l0 6", "M14 11l0 6", "M5 7l1 12a2 2 0 0 0 2 2h8a2 2 0 0 0 2 -2l1 -12", "M9 7v-3a1 1 0 0 1 1 -1h4a1 1 0 0 1 1 1v3"],
    },
    IconDef {
        rust_name: "IconStar",
        tabler_name: "star",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M12 17.75l-6.172 3.245l1.179 -6.873l-5 -4.867l6.9 -1l3.086 -6.253l3.086 6.253l6.9 1l-5 4.867l1.179 6.873l-6.158 -3.245\" />"],
        d_values: &["M0 0h24v24H0z", "M12 17.75l-6.172 3.245l1.179 -6.873l-5 -4.867l6.9 -1l3.086 -6.253l3.086 6.253l6.9 1l-5 4.867l1.179 6.873l-6.158 -3.245"],
    },
    IconDef {
        rust_name: "IconBookmark",
        tabler_name: "bookmark",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M18 7v14l-6 -4l-6 4v-14a4 4 0 0 1 4 -4h4a4 4 0 0 1 4 4\" />"],
        d_values: &["M0 0h24v24H0z", "M18 7v14l-6 -4l-6 4v-14a4 4 0 0 1 4 -4h4a4 4 0 0 1 4 4"],
    },
    IconDef {
        rust_name: "IconMenu2",
        tabler_name: "menu-2",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M4 6l16 0\" />", "<path d=\"M4 12l16 0\" />", "<path d=\"M4 18l16 0\" />"],
        d_values: &["M0 0h24v24H0z", "M4 6l16 0", "M4 12l16 0", "M4 18l16 0"],
    },
    IconDef {
        rust_name: "IconRefresh",
        tabler_name: "refresh",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M20 11a8.1 8.1 0 0 0 -15.5 -2m-.5 -4v4h4\" />", "<path d=\"M4 13a8.1 8.1 0 0 0 15.5 2m.5 4v-4h-4\" />"],
        d_values: &["M0 0h24v24H0z", "M20 11a8.1 8.1 0 0 0 -15.5 -2m-.5 -4v4h4", "M4 13a8.1 8.1 0 0 0 15.5 2m.5 4v-4h-4"],
    },
    IconDef {
        rust_name: "IconAlertTriangle",
        tabler_name: "alert-triangle",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M12 9v4\" />", "<path d=\"M10.363 3.591l-8.106 13.534a1.914 1.914 0 0 0 1.636 2.871h16.214a1.914 1.914 0 0 0 1.636 -2.87l-8.106 -13.536a1.914 1.914 0 0 0 -3.274 0\" />", "<path d=\"M12 16h.01\" />"],
        d_values: &["M0 0h24v24H0z", "M12 9v4", "M10.363 3.591l-8.106 13.534a1.914 1.914 0 0 0 1.636 2.871h16.214a1.914 1.914 0 0 0 1.636 -2.87l-8.106 -13.536a1.914 1.914 0 0 0 -3.274 0", "M12 16h.01"],
    },
    IconDef {
        rust_name: "IconInfoCircle",
        tabler_name: "info-circle",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M3 12a9 9 0 1 0 18 0a9 9 0 0 0 -18 0\" />", "<path d=\"M12 9h.01\" />", "<path d=\"M11 12h1v4h1\" />"],
        d_values: &["M0 0h24v24H0z", "M3 12a9 9 0 1 0 18 0a9 9 0 0 0 -18 0", "M12 9h.01", "M11 12h1v4h1"],
    },
    IconDef {
        rust_name: "IconColumns3",
        tabler_name: "columns-3",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M3 4a1 1 0 0 1 1 -1h16a1 1 0 0 1 1 1v16a1 1 0 0 1 -1 1h-16a1 1 0 0 1 -1 -1v-16\" />", "<path d=\"M9 3v18\" />", "<path d=\"M15 3v18\" />"],
        d_values: &["M0 0h24v24H0z", "M3 4a1 1 0 0 1 1 -1h16a1 1 0 0 1 1 1v16a1 1 0 0 1 -1 1h-16a1 1 0 0 1 -1 -1v-16", "M9 3v18", "M15 3v18"],
    },
    IconDef {
        rust_name: "IconGripVertical",
        tabler_name: "grip-vertical",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M8 5a1 1 0 1 0 2 0a1 1 0 1 0 -2 0\" />", "<path d=\"M8 12a1 1 0 1 0 2 0a1 1 0 1 0 -2 0\" />", "<path d=\"M8 19a1 1 0 1 0 2 0a1 1 0 1 0 -2 0\" />", "<path d=\"M14 5a1 1 0 1 0 2 0a1 1 0 1 0 -2 0\" />", "<path d=\"M14 12a1 1 0 1 0 2 0a1 1 0 1 0 -2 0\" />", "<path d=\"M14 19a1 1 0 1 0 2 0a1 1 0 1 0 -2 0\" />"],
        d_values: &["M0 0h24v24H0z", "M8 5a1 1 0 1 0 2 0a1 1 0 1 0 -2 0", "M8 12a1 1 0 1 0 2 0a1 1 0 1 0 -2 0", "M8 19a1 1 0 1 0 2 0a1 1 0 1 0 -2 0", "M14 5a1 1 0 1 0 2 0a1 1 0 1 0 -2 0", "M14 12a1 1 0 1 0 2 0a1 1 0 1 0 -2 0", "M14 19a1 1 0 1 0 2 0a1 1 0 1 0 -2 0"],
    },
    IconDef {
        rust_name: "IconTag",
        tabler_name: "tag",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M6.5 7.5a1 1 0 1 0 2 0a1 1 0 1 0 -2 0\" />", "<path d=\"M3 6v5.172a2 2 0 0 0 .586 1.414l7.71 7.71a2.41 2.41 0 0 0 3.408 0l5.592 -5.592a2.41 2.41 0 0 0 0 -3.408l-7.71 -7.71a2 2 0 0 0 -1.414 -.586h-5.172a3 3 0 0 0 -3 3\" />"],
        d_values: &["M0 0h24v24H0z", "M6.5 7.5a1 1 0 1 0 2 0a1 1 0 1 0 -2 0", "M3 6v5.172a2 2 0 0 0 .586 1.414l7.71 7.71a2.41 2.41 0 0 0 3.408 0l5.592 -5.592a2.41 2.41 0 0 0 0 -3.408l-7.71 -7.71a2 2 0 0 0 -1.414 -.586h-5.172a3 3 0 0 0 -3 3"],
    },
    IconDef {
        rust_name: "IconPhone",
        tabler_name: "phone",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M5 4h4l2 5l-2.5 1.5a11 11 0 0 0 5 5l1.5 -2.5l5 2v4a2 2 0 0 1 -2 2a16 16 0 0 1 -15 -15a2 2 0 0 1 2 -2\" />"],
        d_values: &["M0 0h24v24H0z", "M5 4h4l2 5l-2.5 1.5a11 11 0 0 0 5 5l1.5 -2.5l5 2v4a2 2 0 0 1 -2 2a16 16 0 0 1 -15 -15a2 2 0 0 1 2 -2"],
    },
    IconDef {
        rust_name: "IconExternalLink",
        tabler_name: "external-link",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M12 6h-6a2 2 0 0 0 -2 2v10a2 2 0 0 0 2 2h10a2 2 0 0 0 2 -2v-6\" />", "<path d=\"M11 13l9 -9\" />", "<path d=\"M15 4h5v5\" />"],
        d_values: &["M0 0h24v24H0z", "M12 6h-6a2 2 0 0 0 -2 2v10a2 2 0 0 0 2 2h10a2 2 0 0 0 2 -2v-6", "M11 13l9 -9", "M15 4h5v5"],
    },
    IconDef {
        rust_name: "IconCopy",
        tabler_name: "copy",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M7 9.667a2.667 2.667 0 0 1 2.667 -2.667h8.666a2.667 2.667 0 0 1 2.667 2.667v8.666a2.667 2.667 0 0 1 -2.667 2.667h-8.666a2.667 2.667 0 0 1 -2.667 -2.667l0 -8.666\" />", "<path d=\"M4.012 16.737a2.005 2.005 0 0 1 -1.012 -1.737v-10c0 -1.1 .9 -2 2 -2h10c.75 0 1.158 .385 1.5 1\" />"],
        d_values: &["M0 0h24v24H0z", "M7 9.667a2.667 2.667 0 0 1 2.667 -2.667h8.666a2.667 2.667 0 0 1 2.667 2.667v8.666a2.667 2.667 0 0 1 -2.667 2.667h-8.666a2.667 2.667 0 0 1 -2.667 -2.667l0 -8.666", "M4.012 16.737a2.005 2.005 0 0 1 -1.012 -1.737v-10c0 -1.1 .9 -2 2 -2h10c.75 0 1.158 .385 1.5 1"],
    },
    IconDef {
        rust_name: "IconKey",
        tabler_name: "key",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M16.555 3.843l3.602 3.602a2.877 2.877 0 0 1 0 4.069l-2.643 2.643a2.877 2.877 0 0 1 -4.069 0l-.301 -.301l-6.558 6.558a2 2 0 0 1 -1.239 .578l-.175 .008h-1.172a1 1 0 0 1 -.993 -.883l-.007 -.117v-1.172a2 2 0 0 1 .467 -1.284l.119 -.13l.414 -.414h2v-2h2v-2l2.144 -2.144l-.301 -.301a2.877 2.877 0 0 1 0 -4.069l2.643 -2.643a2.877 2.877 0 0 1 4.069 0\" />", "<path d=\"M15 9h.01\" />"],
        d_values: &["M0 0h24v24H0z", "M16.555 3.843l3.602 3.602a2.877 2.877 0 0 1 0 4.069l-2.643 2.643a2.877 2.877 0 0 1 -4.069 0l-.301 -.301l-6.558 6.558a2 2 0 0 1 -1.239 .578l-.175 .008h-1.172a1 1 0 0 1 -.993 -.883l-.007 -.117v-1.172a2 2 0 0 1 .467 -1.284l.119 -.13l.414 -.414h2v-2h2v-2l2.144 -2.144l-.301 -.301a2.877 2.877 0 0 1 0 -4.069l2.643 -2.643a2.877 2.877 0 0 1 4.069 0", "M15 9h.01"],
    },
    IconDef {
        rust_name: "IconTerminal",
        tabler_name: "terminal",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M5 7l5 5l-5 5\" />", "<path d=\"M12 19l7 0\" />"],
        d_values: &["M0 0h24v24H0z", "M5 7l5 5l-5 5", "M12 19l7 0"],
    },
    IconDef {
        rust_name: "IconWorld",
        tabler_name: "world",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M3 12a9 9 0 1 0 18 0a9 9 0 0 0 -18 0\" />", "<path d=\"M3.6 9h16.8\" />", "<path d=\"M3.6 15h16.8\" />", "<path d=\"M11.5 3a17 17 0 0 0 0 18\" />", "<path d=\"M12.5 3a17 17 0 0 1 0 18\" />"],
        d_values: &["M0 0h24v24H0z", "M3 12a9 9 0 1 0 18 0a9 9 0 0 0 -18 0", "M3.6 9h16.8", "M3.6 15h16.8", "M11.5 3a17 17 0 0 0 0 18", "M12.5 3a17 17 0 0 1 0 18"],
    },
    IconDef {
        rust_name: "IconChevronsLeft",
        tabler_name: "chevrons-left",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M11 7l-5 5l5 5\" />", "<path d=\"M17 7l-5 5l5 5\" />"],
        d_values: &["M0 0h24v24H0z", "M11 7l-5 5l5 5", "M17 7l-5 5l5 5"],
    },
    IconDef {
        rust_name: "IconLoader",
        tabler_name: "loader",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M12 6l0 -3\" />", "<path d=\"M16.25 7.75l2.15 -2.15\" />", "<path d=\"M18 12l3 0\" />", "<path d=\"M16.25 16.25l2.15 2.15\" />", "<path d=\"M12 18l0 3\" />", "<path d=\"M7.75 16.25l-2.15 2.15\" />", "<path d=\"M6 12l-3 0\" />", "<path d=\"M7.75 7.75l-2.15 -2.15\" />"],
        d_values: &["M0 0h24v24H0z", "M12 6l0 -3", "M16.25 7.75l2.15 -2.15", "M18 12l3 0", "M16.25 16.25l2.15 2.15", "M12 18l0 3", "M7.75 16.25l-2.15 2.15", "M6 12l-3 0", "M7.75 7.75l-2.15 -2.15"],
    },
    IconDef {
        rust_name: "IconBell",
        tabler_name: "bell",
        view_box: "0 0 24 24",
        raw_paths: &["<path stroke=\"none\" d=\"M0 0h24v24H0z\" fill=\"none\" />", "<path d=\"M10 5a2 2 0 1 1 4 0a7 7 0 0 1 4 6v3a4 4 0 0 0 2 3h-16a4 4 0 0 0 2 -3v-3a7 7 0 0 1 4 -6\" />", "<path d=\"M9 17v1a3 3 0 0 0 6 0v-1\" />"],
        d_values: &["M0 0h24v24H0z", "M10 5a2 2 0 1 1 4 0a7 7 0 0 1 4 6v3a4 4 0 0 0 2 3h-16a4 4 0 0 0 2 -3v-3a7 7 0 0 1 4 -6", "M9 17v1a3 3 0 0 0 6 0v-1"],
    },
];

pub const ICON_COUNT: usize = 51;

#[component]
pub fn IconUser(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="user" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconUsers(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="users" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconMail(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="mail" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconLock(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="lock" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconEye(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="eye" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconEyeOff(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="eye-off" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconCheck(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="check" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconX(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="x" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconChevronDown(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="chevron-down" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconChevronUp(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="chevron-up" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconChevronLeft(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="chevron-left" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconChevronRight(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="chevron-right" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconArrowRight(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="arrow-right" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconSun(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="sun" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconMoon(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="moon" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconSettings(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="settings" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconBuilding(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="building" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconBuildingSkyscraper(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="building-skyscraper" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconHome(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="home" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconSearch(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="search" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconPlus(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="plus" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconSparkles(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="sparkles" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconTable(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="table" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconLayoutList(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="layout-list" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconLayoutKanban(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="layout-kanban" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconCalendar(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="calendar" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconFilter(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="filter" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconArrowsSort(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="arrows-sort" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconSelector(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="selector" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconDots(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="dots" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconDotsVertical(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="dots-vertical" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconPencil(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="pencil" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconTrash(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="trash" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconStar(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="star" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconBookmark(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="bookmark" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconMenu2(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="menu-2" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconRefresh(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="refresh" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconAlertTriangle(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="alert-triangle" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconInfoCircle(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="info-circle" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconColumns3(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="columns-3" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconGripVertical(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="grip-vertical" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconTag(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="tag" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconPhone(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="phone" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconExternalLink(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="external-link" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconCopy(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="copy" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconKey(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="key" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconTerminal(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="terminal" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconWorld(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="world" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconChevronsLeft(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="chevrons-left" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconLoader(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="loader" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

#[component]
pub fn IconBell(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {
    view! { <Icon name="bell" size=size stroke=stroke.unwrap_or_default() color=color/> }
}

