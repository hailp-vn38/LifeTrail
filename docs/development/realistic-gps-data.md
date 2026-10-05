# Realistic GPS test dataset

Bộ fixture này dùng để test **server ingestion + Daily View + Web route playback** khi chưa có phần cứng ESP32/GNSS.

## Scenario

Ngày mặc định: `2026-10-05`, timezone `Asia/Ho_Chi_Minh`, bắt đầu `08:00:00`.

Dữ liệu mô phỏng một người đi bộ ở khu trung tâm TP.HCM theo các hành lang đường quanh **Bến Thành → Lê Lợi → Nguyễn Huệ → khu City Hall / Lê Thánh Tôn / Đồng Khởi**. Polyline dùng nhiều anchor theo đường thay vì nối vài waypoint bằng đường thẳng, để line trên MapLibre/MapTiler bám đường hợp lý hơn.

| Local time | Loại | Hành vi |
| --- | --- | --- |
| 08:00:00–08:09:30 | Trip | Bến Thành → Nguyễn Huệ |
| 08:09:30–08:17:30 | Stop | Dừng 8 phút, GPS drift < ~2 m |
| 08:17:30–08:24:00 | Trip | Nguyễn Huệ → City Hall |
| 08:24:00–08:27:00 | Stop | Dừng 3 phút |
| 08:27:00–08:33:00 | Trip | Quay ngược đúng tuyến cũ về Nguyễn Huệ |
| 08:33:00–08:34:15 | Stop | Dừng ngắn 75 giây |
| 08:34:15–08:46:15 | Trip | Đi một vòng block phố |
| 08:46:15–08:52:15 | Stop | Dừng 6 phút |
| 08:52:15–09:02:45 | Trip | Lặp lại đúng vòng block lần 2, pace khác |
| 09:02:45–09:11:45 | Trip | Nguyễn Huệ → Bến Thành, đi lại tuyến ban đầu |
| 09:11:45–09:13:45 | Stop | Dừng cuối hành trình |

Tổng cộng:

- `4426` GPS records tại `1 Hz`.
- `15` Batch, mỗi Batch tối đa 300 record.
- Khoảng `73m45s` timeline thực.
- `6` trip và `5` stop.
- Ground-truth trip distance khoảng `3.62 km`.
- Tốc độ di chuyển được sinh biến thiên, median khoảng `1.14 m/s`, tối đa dưới `1.9 m/s`.
- Có các khoảng giảm tốc/đợi ngắn trong trip để playback không giống robot chạy đều.
- `satellites`, `hdop`, `alt_m`, `course_deg` biến thiên theo thời gian.

## Thành phần

- [`../../tools/generate_realistic_day.py`](../../tools/generate_realistic_day.py): generator độc lập, chỉ dùng Python stdlib.
- `tools/fixtures/realistic-human-day/batches/*.ndjson.ready`: payload `gps/1` có thể upload trực tiếp.
- `tools/fixtures/realistic-human-day/batches/*.manifest`: hash/length/count tương ứng từng Batch.
- `tools/fixtures/realistic-human-day/summary.json`: tổng quan dataset và thứ tự Batch.
- `tools/fixtures/realistic-human-day/daily-view.mock.json`: mock đúng shape Daily View hiện tại để test Web không cần server.
- `tools/fixtures/realistic-human-day/timeline.mock.json`: ground-truth `trips`, `stops`, `timeline_events` theo schema định hướng trong `docs/server/architecture.md`.
- `tools/fixtures/realistic-human-day/route-preview.geojson`: route + stop points để mở nhanh trong GeoJSON viewer/MapLibre.

## Upload vào server thật

Sau khi tạo Owner/Device và lấy device token, chạy từ root repository:

```sh
python3 tools/generate_realistic_day.py \
  --date 2026-10-05 \
  --output ./tools/fixtures/realistic-human-day-runtime \
  --endpoint http://localhost:8080/api/v1/device/batches \
  --token '<lt_dev_token>'
```

Sau đó mở:

```text
http://localhost:8080/devices/<device-uuid>/day/2026-10-05
```

Daily View hiện tại của Phase 1 sẽ thấy toàn bộ raw route và timestamp. Khi playback chạy theo timestamp thực, các stop sẽ làm marker gần như đứng yên trong đúng khoảng thời gian tương ứng.

## Test Web riêng không cần server

`fixture/daily-view.mock.json` có đầy đủ:

- `summary`
- `route.geometry.coordinates`
- `route.properties.timestamps`
- `start`
- `end`

Có thể dùng file này để mock `GET /api/v1/devices/{deviceId}/days/{date}`.

`device_id` trong file mock là UUID placeholder `00000000-0000-0000-0000-000000000001`; khi dùng server thật, server trả UUID Device thực.

## Timeline / Stop / Trip

Phase 1 hiện **chưa materialize** `trips`, `stops`, `timeline_events` vào DB/API. Vì vậy `timeline.mock.json` là ground truth sidecar, không được nhét vào NDJSON vì `gps/1` reject unknown fields.

Khi Phase 2 có processing worker, dùng sidecar này để acceptance test:

- stop detector phải tìm được các stop gần đúng thời gian/center đã định nghĩa;
- trip detector phải giữ được đoạn đi rồi quay lại cùng tuyến;
- loop detector/route rendering phải hiển thị được hai lượt trên cùng một block;
- timeline order phải giữ đúng theo timestamp thực.

## Re-generate cho ngày khác

```sh
python3 tools/generate_realistic_day.py --date 2026-10-06 --output ./tools/fixtures/realistic-human-day-2026-10-06
```

Batch UUID được sinh deterministic theo `date + timezone + batch index`, nên chạy lại cùng tham số cho ra cùng Batch ID và payload, hữu ích để test idempotent replay.

## Kiểm tra đã áp dụng trong bundle

Fixture hiện tại đã được kiểm tra:

- strict LF NDJSON, có final LF;
- không có CRLF;
- timestamp tăng nghiêm ngặt trong từng Batch và toàn ngày;
- đúng 9 field của `gps/1`, không có field lạ;
- SHA-256, `byte_length`, `record_count` trong manifest khớp byte thực;
- Batch lớn nhất ~45 KB, thấp hơn nhiều giới hạn 1 MiB của server.

> Lưu ý: đây là fixture deterministic để test. Route dùng road-aligned anchors tĩnh để không phụ thuộc routing API/network; nó không phải output map-matching centimeter-level từ OSM/MapTiler Directions.
