using System;
using System.Text;
using System.Threading.Tasks;

#if UNITY_WEBGL && !UNITY_EDITOR
using System.Collections;
using System.Runtime.InteropServices;
#else
using System.Collections.Generic;
#endif

public class WebSocket
{
    private readonly Uri _url;

    public void SendString(string str)
    {
        Send(Encoding.UTF8.GetBytes(str));
    }

    public async Task<string> RecvStringAsync()
    {
        var bytes = await RecvAsync();
        return Encoding.UTF8.GetString(bytes);
    }

#if UNITY_WEBGL && !UNITY_EDITOR
    [DllImport("__Internal")]
    private static extern int SocketCreate(string url);

    [DllImport("__Internal")]
    private static extern int SocketState(int socketInstance);

    [DllImport("__Internal")]
    private static extern void SocketSend(int socketInstance, byte[] ptr, int length);

    [DllImport("__Internal")]
    private static extern void SocketRecv(int socketInstance, byte[] ptr, int length);

    [DllImport("__Internal")]
    private static extern int SocketRecvLength(int socketInstance);

    [DllImport("__Internal")]
    private static extern void SocketClose(int socketInstance);

    [DllImport("__Internal")]
    private static extern int SocketError(int socketInstance, byte[] ptr, int length);

    private int m_NativeRef = 0;

    public static Task<WebSocket> ConnectAsync(Uri url)
    {
        throw new NotImplementedException();
    }

    public void Send(byte[] buffer)
    {
        SocketSend(m_NativeRef, buffer, buffer.Length);
    }

    public byte[] Recv()
    {
        var length = SocketRecvLength(m_NativeRef);
        if (length == 0)
        {
            return null;
        }

        var buffer = new byte[length];
        SocketRecv(m_NativeRef, buffer, length);
        return buffer;
    }

    public IEnumerator Connect()
    {
        m_NativeRef = SocketCreate(_url.ToString());

        while (SocketState(m_NativeRef) == 0)
        {
            yield return 0;
        }
    }

    public void Close()
    {
        SocketClose(m_NativeRef);
    }

    public Task<byte[]> RecvAsync()
    {
        return Task.FromResult<byte[]>(null);
    }

    public string error
    {
        get
        {
            const int bufsize = 1024;
            var buffer = new byte[bufsize];
            var result = SocketError(m_NativeRef, buffer, bufsize);

            if (result == 0)
            {
                return null;
            }

            return Encoding.UTF8.GetString(buffer);
        }
    }
#else
    private readonly WebSocketSharp.WebSocket _socket;

    // Queues for tracking pending tasks (i.e. tasks for code that is awaiting a message) and
    // received messages that haven't been dispatched.
    private readonly Queue<TaskCompletionSource<byte[]>> _pendingTasks = new Queue<TaskCompletionSource<byte[]>>();
    private readonly Queue<byte[]> _pendingMessages = new Queue<byte[]>();

    private WebSocket(Uri url, WebSocketSharp.WebSocket socket)
    {
        _url = url;
        _socket = socket;

        // Register the listeners for incoming messages/errors.
        _socket.OnMessage += OnMessage;
        _socket.OnError += OnError;
    }

    /// <summary>
    /// Attempts to create a web socket connected to <paramref name="url"/>, returning the socket
    /// once the connection was successfully establish. Throws an exception if the connection
    /// fails.
    /// </summary>
    ///
    /// <param name="url">The endpoint to connect to.</param>
    ///
    /// <returns>The web socket once the connection has been established.</returns>
    ///
    /// <exception cref="WebSocketException">
    /// Throws an exception if the connection fails or if an error occurs before the connection
    /// can be established.
    /// </exception>
    public static Task<WebSocket> ConnectAsync(Uri url)
    {
        var protocol = url.Scheme;
        if (protocol != "ws" && protocol != "wss")
        {
            throw new ArgumentException("Unsupported protocol: " + protocol);
        }

        // Create a completion source so that we can return the finished web socket
        // asynchronously.
        var completion = new TaskCompletionSource<WebSocket>();

        // Create the underlying socket and setup callbacks to either yield the connected
        // WebSocket or return an error if the connection fails.
        var socket = new WebSocketSharp.WebSocket(url.ToString());
        socket.OnOpen += (sender, e) =>
        {
            completion.TrySetResult(new WebSocket(url, socket));
        };
        socket.OnError += (sender, args) =>
        {
            completion.TrySetException(new WebSocketException(args.Message));
        };

        // Begin the connection and return the task.
        socket.ConnectAsync();
        return completion.Task;
    }

    public void Send(byte[] buffer)
    {
        _socket.Send(buffer);
    }

    /// <summary>
    /// Returns the next message received on the socket as a byte array.
    /// </summary>
    ///
    /// <returns>The next message received on the socket.</returns>
    ///
    /// <exception cref="WebSocketException">
    /// Throws an exception if an error occurs while waiting for an incoming message,
    /// or if the socket disconnects.
    /// </exception>
    ///
    /// <remarks>
    /// This method guarantees that each incoming message will only be yielded once,
    /// and that messages will be yielded in the order they are received. It is
    /// therefor safe to call this method multiple times without waiting for each to
    /// task to resolve.
    /// </remarks>
    public Task<byte[]> RecvAsync()
    {
        // If we've already received a message that hasn't been dispatched, immediately resolve
        // the task with the first message in the queue. Otherwise, create a TaskCompletionSource
        // so that we can resolve the task with the next message we receive.
        if (_pendingMessages.Count > 0)
        {
            return Task.FromResult(_pendingMessages.Dequeue());
        }
        else
        {
            var completion = new TaskCompletionSource<byte[]>();
            _pendingTasks.Enqueue(completion);
            return completion.Task;
        }
    }

    private void OnMessage(object sender, WebSocketSharp.MessageEventArgs args)
    {
        if (_pendingTasks.Count > 0)
        {
            var completion = _pendingTasks.Dequeue();
            completion.TrySetResult(args.RawData);
        }
        else
        {
            _pendingMessages.Enqueue(args.RawData);
        }
    }

    private void OnError(object sender, WebSocketSharp.ErrorEventArgs args)
    {
        // Have all pending tasks fail when an error occurs.
        //
        // TODO: Is there a better way to handle this? Maybe only fail the first pending task?
        // We should either switch to a better approach, or explain why this approach is ideal.
        foreach (var completion in _pendingTasks)
        {
            completion.TrySetException(new WebSocketException(args.Message));
        }
        _pendingTasks.Clear();
    }


    public void Close()
    {
        foreach (var completion in _pendingTasks)
        {
            completion.TrySetCanceled();
        }
        _pendingTasks.Clear();

        _socket.CloseAsync();
    }
#endif
}

public class WebSocketException : Exception
{
    public WebSocketException() { }

    public WebSocketException(string message) : base(message) { }
}
