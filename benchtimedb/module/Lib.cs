using System.Runtime.InteropServices.ComTypes;
using SpacetimeDB;

public static partial class Module
{
    [Table(Name = "Logs", Public = true)]
    public partial struct Logs
    {
        [PrimaryKey] [AutoInc] public uint Id;
        public double Sent;
        public double Received;
        public double Latency;
        public double Jitter;
        public bool UnderLoad;
    }
    
    [Table(Name = "ClockSync", Public = true)]
    public partial struct ClockSync
    {
        [PrimaryKey] public Identity Identity;
        public double Clock;
    }

    [Table(Name = "Data", Public = true)]
    public partial struct Data
    {
        [PrimaryKey] [AutoInc] public uint Id;
        public int[] data;
    }

    [Table(Name = "DeleteData", Scheduled = nameof(DeleteDataWorker), ScheduledAt = nameof(ScheduledAt), Public = false)]
    public partial struct DeleteData
    {
        [PrimaryKey] [AutoInc] public ulong Id;
        public ScheduleAt ScheduledAt;
        public uint DataId;
    }

    [Reducer(ReducerKind.ClientConnected)]
    public static void Connect(ReducerContext ctx)
    {
        var existingClient = ctx.Db.ClockSync.Identity.Find(ctx.Sender);
        if (existingClient is not null)
        {
            ctx.Db.ClockSync.Delete(existingClient.Value);
        }
        
        ctx.Db.ClockSync.Insert(new ClockSync
        {
            Identity = ctx.Sender,
            Clock = ctx.Timestamp.ToStd().ToUnixTimeSeconds()
        });
    }

    [Reducer]
    public static void DeleteDataWorker(ReducerContext ctx, DeleteData deleteData)
    {
        if (ctx.Sender != ctx.Identity) return;
        var data = ctx.Db.Data.Id.Find(deleteData.DataId);
        if (data is null) return;

        ctx.Db.Data.Delete(data.Value);
    }

    [Reducer]
    public static void AddData(ReducerContext ctx, int[] data)
    {
        var d = ctx.Db.Data.Insert(new Data
        {
            data = data
        });

        ctx.Db.DeleteData.Insert(new DeleteData
        {
            ScheduledAt = ctx.Timestamp.ToStd().AddSeconds(0),
            DataId = d.Id
        });
    }

    [Reducer]
    public static void AddLog(ReducerContext ctx, double sent, bool underLoad)
    {
        double received = (ctx.Timestamp.ToStd() - DateTime.UnixEpoch).TotalSeconds;

        if (ctx.Db.Logs.Count == 0)
        {
            ctx.Db.Logs.Insert(new Logs
            {
                Sent = sent,
                Received = received,
                Latency = 0,
                Jitter = 0,
                UnderLoad = underLoad
            });
            return;
        }

        var latestLog = ctx.Db.Logs.Iter().OrderByDescending(l => l.Id).First();
        double latency = Math.Abs(received - sent);
        double jitter = latestLog.Latency == 0 ? 0 : Math.Abs(latency - latestLog.Latency);

        ctx.Db.Logs.Insert(new Logs
        {
            Sent = sent,
            Received = received,
            Latency = latency,
            Jitter = jitter,
            UnderLoad = underLoad
        });
    }
}
