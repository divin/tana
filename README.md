# Tana - Media Tracking CLI Tool

Tana (Japanese (棚) for "shelf") is a lightweight Rust CLI tool for tracking movies, TV series, and books you've consumed. Built with SQLite for persistent storage and designed with extensibility in mind for future media types. I built it mainly for myself to keep track of my media consumption, but it's open source and contributions are welcome!

## AI Disclaimer

This project was developed with the assistance of GitHub Copilot, an AI code generation tool. I know this is a controversial topic. While Copilot helped me speed up development and provided useful suggestions, all code was reviewed and edited by me to ensure quality and correctness. The final implementation reflects my design choices and coding style. Below is a bit of my take on using AI tools in software development, if you are interested.

<details>
  <summary>My Take on AI Assistance</summary>
  
  > I started programming in 2018 and have therefore been coding for nearly 10 years in various languages (mainly Python, Lua, C++, and Rust). I'm still learning every day, but I would argue I have a fairly good understanding of programming concepts, design patterns, and best practices. I'm not particularly interested in vibe-coding (specifically the part where AI writes code without human review), but I see AI tools like Copilot as powerful assistants that can help with boilerplate code, suggest solutions, and speed up development. As long as the developer maintains control, reviews the code, and ensures it meets quality standards, I think AI can be a valuable tool in the software development process. It's not about replacing developers but augmenting their capabilities. In this project, I used Copilot to generate code snippets, but I made sure to review and edit everything to ensure it aligned with my vision for the project. The end result is a product of both human creativity and AI assistance.
  
</details>

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Troubleshooting

### Database Not Found
If you get a "database not found" error, the database will be created automatically on the first run. Make sure the `~/.local/share/tana/` directory exists and is writable.

### Permission Denied
```bash
chmod 755 ~/.local/share/tana/
```

### Clear All Data
```bash
rm ~/.local/share/tana/tana.db
```

The database will be recreated with a fresh schema on the next run.


## Contributing

Contributions are welcome! Please ensure:
- All tests pass: `cargo test`
- No warnings: `cargo build`
- Code is formatted: `cargo fmt`
- Clippy is happy: `cargo clippy`


### Debug Information
Run with the `--debug` flag to see detailed logs:
```bash
tana --debug show movies
```

---

Made with ❤️ in Rust with support from GitHub Copilot. Happy tracking!
