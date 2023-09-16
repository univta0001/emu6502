To prepare an .a2stream file for streaming:

    Create a header-less .raw file with 22050Hz mono 32-bit-float PCM data (e.g. with Audacity)
    Generate an .a2stream file from the .raw file with gena2stream (source code)
        Put a standard 16kB .dhgr file beside the .raw file for custom cover art (optional)
        Use the option to -p switch the visualization from level meter to progress bar
    Put the .a2stream file onto any HTTP (not HTTPS) server
        Run a simple local HTTP server on Windows
            Run the HTTP File Server and drop the file you want to stream in its Virtual File System
        Run a simple local HTTP server on Linux
            cd to the directory containing the file you want to stream and enter python -m SimpleHTTPServer or python3 -m http.server depending on the Python version you want to use

