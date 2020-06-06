using System;

namespace Synapse.Mahjong
{
    /// <summary>
    /// Exception thrown when the client's local state is out of sync with the server.
    /// </summary>
    ///
    /// <remarks>
    /// This exception should specifically be used to indicate a situation where the
    /// local state cannot be smoothly reconciled with the server state. In these cases
    /// it's necessary to perform a more heavy-handed reset of the client state to match
    /// the latest server state.
    /// </remarks>
    public class OutOfSyncException : Exception
    {
        public OutOfSyncException() : base() { }

        public OutOfSyncException(string message) : base(message) { }
    }
}
