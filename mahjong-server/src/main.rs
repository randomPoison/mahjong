use warp::Filter;

#[tokio::main]
async fn main() {
    // GET /hello/warp => 200 OK with body "Hello, warp!"
    
    let client = warp::path("client").and(warp::ws()).map(|ws: warp::ws::Ws| {
        ws.on_upgrade(move |_socket| async { todo!(); })
    });

    let index = warp::path::end().map(|| warp::reply::html(INDEX_HTML));

    let routes = index.or(client);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030))
        .await;
}

static INDEX_HTML: &str = r#"
<!DOCTYPE html>
<html>
    <head>
        <title>Warp Chat</title>
    </head>
    <body>
        <h1>warp chat</h1>
        <div id="chat">
            <p><em>Connecting...</em></p>
        </div>
        <input type="text" id="text" />
        <button type="button" id="send">Send</button>
        <script type="text/javascript">
        var uri = 'ws://' + location.host + '/client';
        var ws = new WebSocket(uri);
        function message(data) {
            var line = document.createElement('p');
            line.innerText = data;
            chat.appendChild(line);
        }
        ws.onopen = function() {
            chat.innerHTML = "<p><em>Connected!</em></p>";
        }
        ws.onmessage = function(msg) {
            message(msg.data);
        };
        send.onclick = function() {
            var msg = text.value;
            ws.send(msg);
            text.value = '';
            message('<You>: ' + msg);
        };
        </script>
    </body>
</html>
"#;
