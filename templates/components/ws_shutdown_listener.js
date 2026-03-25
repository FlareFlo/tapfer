function ws_shutdown_listener(ws) {
    ws.addEventListener("message", (e) => {
        const payload = JSON.parse(e.data);
        switch (payload.event.key) {
            case "Shutdown":
                show_toast("Server restarting/shutting down", "error");
                // Give server time to shut down
                setTimeout(() => {
                    pollServer()
                }, 1000);
                break;
        }
    });
}

function pollServer() {
    setInterval(() => {
        console.log("Waiting for server to return");
        show_toast("Lost connection", "error");
        fetch(window.location.href, { method: "HEAD" })
            .then(() => window.location.reload())
            .catch(() => {});
    }, 500);
}
