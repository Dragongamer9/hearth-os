"""Read-only Linux telemetry. /proc and /sys only; no privileged actions."""
from pathlib import Path
import threading

_LOCK=threading.Lock()
_LAST_CPU=None

def _text(path):
    try: return Path(path).read_text(encoding="utf-8",errors="replace")
    except (OSError,ValueError): return ""

def _cpu_times():
    for line in _text("/proc/stat").splitlines():
        if line.startswith("cpu "):
            try:
                v=[int(n) for n in line.split()[1:]]
                if len(v)<4: return None
                return (sum(v),v[3]+(v[4] if len(v)>4 else 0))
            except ValueError: return None
    return None

def _cpu_percent():
    global _LAST_CPU
    current=_cpu_times()
    with _LOCK:
        previous=_LAST_CPU
        _LAST_CPU=current
    if current is None or previous is None: return None
    total=current[0]-previous[0]
    if total<=0: return None
    return round(max(0,min(100,100*(1-(current[1]-previous[1])/total))),1)

def _memory():
    values={}
    for line in _text("/proc/meminfo").splitlines():
        if ":" not in line: continue
        k,v=line.split(":",1)
        try: values[k]=int(v.strip().split()[0])
        except (ValueError,IndexError): pass
    total=values.get("MemTotal",0)
    available=values.get("MemAvailable",values.get("MemFree",0))
    if not total: return {"usedGiB":None,"totalGiB":None,"percent":None}
    used=max(0,total-available)
    return {"usedGiB":round(used/1048576,2),"totalGiB":round(total/1048576,2),"percent":round(100*used/total,1)}

def _temperatures():
    temps=[]
    for zone in Path("/sys/class/thermal").glob("thermal_zone*"):
        try:
            value=float((zone/"temp").read_text().strip())/1000
            label=(zone/"type").read_text().strip()
            if -20<=value<=130: temps.append({"name":label[:64],"celsius":round(value,1)})
        except (OSError,ValueError): pass
    return temps[:24]

def _gpu():
    devices=[]
    for card in Path("/sys/class/drm").glob("card[0-9]*"):
        try: driver=(card/"device"/"driver").resolve(strict=True).name
        except (OSError,RuntimeError): continue
        if driver not in devices: devices.append(driver)
    return devices

def snapshot():
    return {"cpuPercent":_cpu_percent(),"memory":_memory(),"thermalSensors":_temperatures(),
            "gpuDrivers":_gpu(),"fps":None,"gpuPercent":None,
            "notes":"CPU is a sampled average. FPS and GPU utilization require host integration."}
