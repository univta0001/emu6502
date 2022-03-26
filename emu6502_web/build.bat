@echo off
@wasm-pack build --release --target web
@npx tailwindcss -o style.css --minify
