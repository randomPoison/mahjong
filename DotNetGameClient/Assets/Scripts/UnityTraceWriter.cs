using System;
using System.Diagnostics;
using Newtonsoft.Json.Serialization;

/// <summary>
/// Logs Json.NET traces to the Unity log console.
/// </summary>
public class UnityTraceWriter : ITraceWriter
{
    public TraceLevel LevelFilter
    {
        // Log all messages, Unity has its own filtering setup.
        get { return TraceLevel.Verbose; }
    }

    public void Trace(TraceLevel level, string message, Exception ex)
    {
        if (ex != null) {
            UnityEngine.Debug.LogException(ex);
        }

        switch (level)
        {
            case TraceLevel.Error:
                UnityEngine.Debug.LogError(message);
                break;
            case TraceLevel.Info:
            case TraceLevel.Verbose:
                UnityEngine.Debug.Log(message);
                break;
            case TraceLevel.Warning:
                UnityEngine.Debug.LogWarning(message);
                break;
        }
    }
}
