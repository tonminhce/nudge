use front_ui::tokens::{dark_count, generate_css, light_count, Scheme};

fn main() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let out_dir = std::path::Path::new(manifest_dir)
        .join("theme")
        .join("generated");
    std::fs::create_dir_all(&out_dir).expect("cannot create theme/generated");
    std::fs::write(
        out_dir.join("tokens-light.css"),
        generate_css(Scheme::Light),
    )
    .expect("cannot write tokens-light.css");
    std::fs::write(out_dir.join("tokens-dark.css"), generate_css(Scheme::Dark))
        .expect("cannot write tokens-dark.css");
    println!(
        "generated tokens-light.css ({} tokens) and tokens-dark.css ({} tokens)",
        light_count(),
        dark_count()
    );
}
