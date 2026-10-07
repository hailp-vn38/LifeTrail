# Đề xuất xuất video Daily View trên web

Status: implemented — MVP dùng MediaRecorder, menu Thao tác trong header

## Khuyến nghị

Bản đầu tiên xuất trong trình duyệt bằng canvas composition + MediaRecorder. Không thêm API, worker hay container video. Tạo một MapLibre instance riêng cho phiên xuất, tái sử dụng các module playback/layer hiện có. Video mặc định 1280×720, ngang, mục tiêu 30 FPS, không âm thanh. Chọn định dạng theo khả năng thực tế của browser; ưu tiên MP4 khi được hỗ trợ, fallback WebM và ghi đúng phần mở rộng.

Đây là giải pháp ghi theo thời gian thực: video dài hai phút cần khoảng hai phút ghi, cộng khởi tạo và hoàn tất file. Không cam kết FPS cố định trên máy yếu. Nếu yêu cầu bắt buộc MP4, xuất theo từng frame, hoặc chạy khi đóng tab, cần chuyển sang phương án khác.

## Bằng chứng từ project

- `web/src/features/daily-map/components/DailyMapActions.vue`: menu Xuất hiện có GPX/CSV, là vị trí thêm Video.
- `web/src/components/RouteMap.vue`: Vue component đang điều phối MapLibre, controller, camera, store và lifecycle. Không tiếp tục gom pipeline video vào file này.
- `web/src/map/route-playback/controller.ts`: có scheduler injectable và seek; tái sử dụng tính vị trí theo thời gian, nhưng state playing/paused, camera easing và timer hiện tại chưa phải pipeline render offline.
- `web/src/map/route-playback/parts.ts`, `timeline.ts`, `progress.ts`: dữ liệu Route Parts, progress anchors và xử lý GPS Gap/Evidence Hole có sẵn. Cần giữ các ngắt đoạn, không tạo đường nối giả.
- `web/src/map/route-playback/layers.ts`: heading puck và route nằm trong MapLibre canvas. `web/src/map/event-dots.ts` có circle/symbol layers nhưng popup là DOM.
- PlaybackBar và thông báo interruption nằm ngoài canvas. Canvas video phải vẽ đồng hồ, caption và attribution riêng.
- Daily playback dùng dayClockBounds; play bắt đầu tại quan sát đầu nhưng controller có thể chạy tới cuối ngày. Phiên xuất nên có khoảng thời gian độc lập.
- `deploy/README.md`, `deploy/docker-compose.yml`: web chạy HTTP qua LAN, Nginx + Rust API + Postgres. WebCodecs yêu cầu secure context; HTTP LAN hiện tại không phải nền tảng phù hợp để chọn nó làm đường xuất duy nhất.
- ADR-0004/ADR-0007: anchors và server-owned distance vẫn authoritative; xuất chỉ trình bày Published Daily Snapshot, không xử lý lại GPS hay gọi routing engine.

## Luồng người dùng đề xuất

1. Xuất → Video mở hộp cấu hình và preview.
2. Mặc định lấy từ quan sát Route đầu đến cuối, có thể chọn khoảng ngắn hơn. Báo không xuất được khi không có nguồn playback hợp lệ.
3. Chọn tốc độ 1x/10x/50x/100x, hiển thị thời lượng dự kiến trước khi bắt đầu; cảnh báo trong hộp cấu hình nếu video dài. Ví dụ tám giờ ở 100x vẫn là 4 phút 48 giây, chưa tính intro/outro.
4. Chọn camera Tổng quan cố định hoặc Theo hành trình. Nên mặc định Tổng quan cho toàn ngày; góc follow sát đường ở tốc độ cao có thể chuyển động mạnh.
5. Preview dùng style đang chọn, nội dung gồm bản đồ, route tiến dần, vị trí, ngày/giờ theo timezone Owner, trạng thái khoảng thiếu quan sát và attribution nguồn bản đồ. Không ghi các nút điều khiển.
6. Bắt đầu → tiến độ + Hủy → tải file sau khi recorder đã phát chunk cuối. Yêu cầu giữ tab hoạt động; bản đầu có thể hủy có thông báo khi tab bị ẩn, tránh âm thầm tạo video thiếu frame.

Giữ thời lượng Stop và khoảng thiếu quan sát theo tỷ lệ tốc độ trong MVP. Không kéo marker qua GPS Gap/Evidence Hole. Nén các khoảng đó để tạo clip 30/60/120 giây là tính năng riêng sau này, cần ánh xạ thời gian video ↔ thời gian GPS và caption chuyển cảnh rõ ràng.

## Kiến trúc đề xuất

Đặt dưới `web/src/features/video-export/`, mỗi module một trách nhiệm:

- `components/VideoExportDialog.vue`: cấu hình, preview, tiến độ, hủy.
- `composables/useVideoExport.ts`: lifecycle phiên xuất và trạng thái preparing/recording/finalizing/done/error/cancelled.
- `export-plan.ts`: kiểm tra khoảng thời gian, tốc độ, thời lượng và giới hạn kích thước.
- `map-renderer.ts`: MapLibre instance riêng, style, layers, controller/camera riêng, cleanup; không dùng playback store toàn cục.
- `compositor.ts`: copy map canvas và vẽ overlay vào canvas 2D cố định kích thước.
- `media-recorder.ts`: feature detection, MIME candidates, recorder events, chunks, Blob và dừng tracks.

Snapshot đầu vào và cấu hình được chốt lúc bắt đầu. Refresh, đổi ngày hay đổi style của giao diện không được thay đổi nội dung một phiên xuất đang chạy. Giới hạn một phiên xuất; navigation/unmount phải hủy và giải phóng WebGL, timer, stream, URL.

Map export dùng vùng render có kích thước thật, không dùng `display:none`. Khi bản đồ render xong, compositor copy canvas ngay trong callback render để tránh đọc WebGL buffer sau khi bị clear. Chỉ cân nhắc `preserveDrawingBuffer` trên map export nếu kiểm chứng cho thấy cần, vì có chi phí hiệu năng. Bắt đầu ghi sau khi style/tiles/glyphs ban đầu đã sẵn sàng, có timeout/lỗi rõ ràng. Camera di chuyển vẫn có thể tải tile mới: phải kiểm thử thực tế; chờ từng frame khi tile chậm thuộc pipeline deterministic sau này.

Không lưu toàn bộ bitmap frames vào RAM. Nhận encoded chunks định kỳ; nếu gom Blob trong RAM thì tổng bộ nhớ vẫn tăng theo độ dài video. Giới hạn độ dài/bitrate cho MVP và hiển thị kích thước ước tính. Đổi đuôi WebM thành MP4 không phải chuyển đổi định dạng.

## So sánh các phương án

| Phương án | Điểm mạnh | Giới hạn | Vai trò |
| --- | --- | --- | --- |
| Canvas + MediaRecorder | Ít thay đổi hạ tầng, tái sử dụng playback/camera | Ghi theo thời gian thực, chất lượng phụ thuộc máy/tab, codec tùy browser | MVP đề xuất |
| WebCodecs + muxer | Chủ động timestamp/frame; có thể render nhanh hoặc chậm hơn thời gian video | Secure context, kiểm tra codec, thêm muxer/backpressure, camera phải theo thời gian ảo | Nâng cấp khi cần FPS/thời lượng chính xác |
| Headless browser + FFmpeg ở server | Có thể tạo MP4 thống nhất, chạy background | Thêm job/storage/container, tài nguyên GPU/CPU, quyền truy cập snapshot và style | Khi cần xuất dài hoặc đóng tab vẫn tiếp tục |

## Kiểm chứng trước khi xây đầy đủ

Làm spike trên browser desktop thực tế và origin HTTP LAN hiện tại: map 2D/3D/vệ tinh → composition → recorder → file phát được. Kiểm tra canvas đen/CORS, attribution, codec và tile tải khi camera chạy. Spike là điều kiện đánh giá tính khả thi, không coi API tồn tại là bảo đảm video tốt.

Khi triển khai: kiểm thử tính thời lượng/khoảng thời gian, Route Part boundaries và interruption; hủy/lỗi/cleanup; file đúng container; browser smoke test trên HTTP LAN và HTTPS với route thật. Chưa cần đổi deploy cho MVP.

## Nguồn API

- [Media capture from DOM elements](https://w3c.github.io/mediacapture-fromelement/#html-canvas-element-media-capture): canvas capture và origin-clean.
- [MediaStream Recording](https://www.w3.org/TR/mediastream-recording/): recorder, MIME support và chunk events.
- [WebCodecs](https://www.w3.org/TR/webcodecs/): VideoEncoder, secure context và encoded chunks.
- [MapLibre MapOptions](https://maplibre.org/maplibre-gl-js/docs/API/type-aliases/MapOptions/): canvas context và preserveDrawingBuffer.
- Xem thêm [browser-research.md](browser-research.md) để có chi tiết kiểm chứng và nguồn bổ sung.
