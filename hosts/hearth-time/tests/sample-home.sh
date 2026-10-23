#!/bin/sh
# Fills a HEARTH_TIME_HOME folder with a few alarms, events, notes and a cached forecast, so CI's screenshots of the
# real apps show something. Usage: sh tests/sample-home.sh <folder>
set -e
H="$1"
[ -n "$H" ] || { echo "usage: $0 <folder>"; exit 2; }
mkdir -p "$H/config/hearth-clock" "$H/data/hearth-calendar" "$H/Documents/Notes" "$H/config/hearth-weather" "$H/cache/hearth-weather"
now=$(date +%s%3N)
today=$(date +%F); tomorrow=$(date -d tomorrow +%F); in3=$(date -d '+3 days' +%F)
cat > "$H/config/hearth-clock/clock.json" <<EOF
{"alarms": [
  {"id": "a1", "time": "06:45", "days": [1,2,3,4,5], "label": "Work", "enabled": true, "since": 0, "snooze": 10},
  {"id": "a2", "time": "09:30", "days": [0,6], "label": "Weekend", "enabled": true, "since": 0}],
 "timers": [{"id": "t1", "label": "Pasta", "duration": 600000, "endsAt": $((now + 420000)), "remaining": 600000},
            {"id": "t2", "label": "Tea", "duration": 240000, "endsAt": null, "remaining": 95000}],
 "stopwatch": {"startedAt": null, "elapsed": 83450, "laps": [26100, 54300, 83450]},
 "world": [{"name": "Tokyo", "tz": "Asia/Tokyo"}, {"name": "New York", "tz": "America/New_York"}],
 "settings": {"ringMinutes": 10, "snooze": 10}}
EOF
cat > "$H/data/hearth-calendar/calendar.json" <<EOF
{"events": [
  {"id": "e1", "calendar": "local", "title": "Standup", "start": "${today}T09:30:00", "end": "${today}T09:45:00", "rrule": "FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR", "reminders": [5]},
  {"id": "e2", "calendar": "local", "title": "Dentist", "start": "${today}T15:00:00", "end": "${today}T16:00:00", "location": "Smile Clinic", "reminders": [60]},
  {"id": "e3", "calendar": "local", "title": "Lunch with Sam", "start": "${tomorrow}T12:30:00", "end": "${tomorrow}T13:30:00", "reminders": [10]},
  {"id": "e4", "calendar": "local", "title": "Mum's birthday", "allDay": true, "start": "${in3}", "end": "$(date -d '+4 days' +%F)", "rrule": "FREQ=YEARLY", "reminders": [0]}],
 "subs": [], "settings": {"weekStart": 1, "view": "month", "defaultReminder": 10}}
EOF
printf '# Shopping\n- [x] Eggs\n- [ ] Oat milk\n- [ ] Coffee beans\n' > "$H/Documents/Notes/Shopping.md"
printf '# Game backlog\n**Playing:** Elden Ring\n\n- Hollow Knight: Silksong\n- *Outer Wilds* (again)\n\n> Finish one before buying another.\n' > "$H/Documents/Notes/Game backlog.md"
printf 'Shopping.md\n' > "$H/Documents/Notes/.pinned"
# Weather: a saved place and a forecast fetched "just now", so the window draws without the network.
echo '{"place": {"name": "Lisbon", "admin1": "Lisbon", "country": "Portugal", "latitude": 38.72, "longitude": -9.13, "timezone": "Europe/Lisbon"}, "units": "metric"}' > "$H/config/hearth-weather/weather.json"
python3 - "$H/cache/hearth-weather/forecast.json" "$now" <<'PY'
import json, sys, datetime as dt
path, now = sys.argv[1], int(sys.argv[2])
t0 = dt.datetime.now().replace(minute=0, second=0, microsecond=0)
hours = [t0 + dt.timedelta(hours=i) for i in range(48)]
codes = [0, 1, 2, 2, 3, 61, 80, 3, 2, 1, 0, 0]
f = {"fetchedAt": now, "units": "metric",
     "place": {"name": "Lisbon", "latitude": 38.72, "longitude": -9.13},
     "current": {"time": t0.strftime("%Y-%m-%dT%H:%M"), "temperature_2m": 18.4, "apparent_temperature": 17.1, "relative_humidity_2m": 64, "weather_code": 2, "is_day": 1, "wind_speed_10m": 14, "wind_direction_10m": 300, "precipitation": 0},
     "current_units": {"wind_speed_10m": "km/h", "precipitation": "mm"},
     "hourly": {"time": [h.strftime("%Y-%m-%dT%H:%M") for h in hours], "temperature_2m": [14 + (i % 12) / 2 for i in range(48)],
                "weather_code": [codes[i % 12] for i in range(48)], "precipitation_probability": [[0, 0, 5, 10, 20, 70, 55, 20, 5, 0, 0, 0][i % 12] for i in range(48)],
                "is_day": [1 if 7 <= h.hour < 19 else 0 for h in hours]},
     "daily": {"time": [(t0 + dt.timedelta(days=i)).strftime("%Y-%m-%d") for i in range(7)], "weather_code": [2, 61, 80, 3, 1, 0, 95],
               "temperature_2m_max": [21, 17, 16, 18, 22, 24, 20], "temperature_2m_min": [13, 12, 11, 11, 13, 15, 14], "precipitation_probability_max": [10, 80, 65, 20, 0, 0, 60],
               "sunrise": [t0.strftime("%Y-%m-%dT07:41")] * 7, "sunset": [t0.strftime("%Y-%m-%dT18:36")] * 7, "uv_index_max": [4, 2, 2, 3, 5, 5, 3]}}
json.dump(f, open(path, "w"))
PY
echo "sample home in $H"
