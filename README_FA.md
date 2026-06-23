<div align="center">

# 🌐 IRBox Client

![IRBox Screenshot](screenshot.png)

**اپلیکیشن IRBox یک کلاینت پروکسی انعطاف‌پذیر و امن است که با فناوری‌های مدرن ساخته شده تا اتصال اینترنتی بی‌دردسر و قابل اعتماد را فراهم کند**

این نرم‌افزار برای کاربران آگاه از حریم خصوصی طراحی شده و از پشتیبانی چند پروتکلی، قابلیت‌های مسیریابی پیشرفته و ابزارهای مدیریتی ساده برخوردار است تا تجربه مرور امن و بدون مشکلی را تضمین کند.

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE) 
[![Releases](https://img.shields.io/github/downloads/frank-vpl/IRBox/total.svg)](https://github.com/frank-vpl/IRBox/releases/latest)
[![Latest Release](https://img.shields.io/github/v/release/frank-vpl/IRBox)](https://github.com/frank-vpl/IRBox/releases/latest)

[English Version](README.md)

</div>

## 🚀 ویژگی‌های کلیدی

### پشتیبانی چند پروتکلی
- **VLESS**
- **VMess**
- **Shadowsocks**
- **Trojan**
- **Hysteria2**
- **TUIC**
- **SSH**
- **WireGuard**

### مدیریت پیشرفته
- **پشتیبانی از اشتراک** - درون‌ریزی و به‌روزرسانی خودکار لینک‌های اشتراک
- **قوانین مسیریابی** - قوانین مبتنی بر دامنه (پروکسی/مستقیم/مسدود/واسط) با پیش‌تنظیماتی برای مسدودسازی تبلیغات و دور زدن منطقه‌ای
- **تونل‌زنی تقسیم** - انتخاب مسیر پیش‌فرض: تمام ترافیک یا دامنه‌های انتخابی را پروکسی کنید
- **مسیریابی واسط سفارشی** - هدایت دامنه‌های انتخابی به یک واسط شبکه که خودتان مدیریت می‌کنید (مثلاً یک تونل WireGuard/AmneziaWG)

### حالت‌های اتصال
- **پروکسی سیستم** - پروکسی HTTP برای دسترسی سراسر سیستم
- **حالت TUN** - VPN کامل که تمام ترافیک را ضبط می‌کند
- **ارتقاء مدیر** - "اجرای با عنوان مدیر" با یک کلیک برای حالت TUN

### تجربه کاربری
- **آشنایی اولیه** - تور تعاملی راهنما برای کاربران جدید
- **پینگ TCP** - تست تاخیر انبوه سرورها
- **انتخاب خودکار بهترین سرور** - انتخاب هوشمند سرور
- **تم‌ها** - ۲ تم رنگی (تیره، روشن)
- **سبک‌ها** - پیش‌فرض، حداقلی

## 🔀 مسیریابی واسط سفارشی

IRBox می‌تواند دامنه‌های انتخابی را به یک واسط شبکه که **خودتان آن را بالا می‌آورید و مدیریت می‌کنید** هدایت کند — برای مثال یک تونل WireGuard/AmneziaWG که با `table = off` ساخته شده است. IRBox این واسط را ایجاد یا حذف نمی‌کند؛ تنها ترافیک منطبق را از طریق sing-box به آن هدایت می‌کند. این قابلیت فقط مخصوص sing-box است (با هسته Xray، کنش «واسط» به «پروکسی» تنزل می‌یابد).

**نحوه استفاده:**

1. واسط خود را خارج از IRBox بالا بیاورید (مثلاً `awg0` / `wg0`). در لینوکس آن را با `table = off` و علامت فایروال (fwmark) مخصوص خودش پیکربندی کنید تا سیستم‌عامل به‌طور خودکار همه‌چیز را به داخل آن مسیریابی نکند.
2. در IRBox صفحهٔ **مسیریابی** را باز کنید و بخش **مسیریابی واسط سفارشی** را بیابید:
   - **نام واسط** — واسطی که باید به آن متصل شود، مثلاً `awg0`.
   - **آی‌پی‌های مقصد برای استثنا** — آی‌پی(های) سرور تونل، جداشده با کاما. در حالت TUN این‌ها روی مسیر مستقیم نگه داشته می‌شوند تا دست‌دهی (handshake) خودِ تونل دوباره به داخل sing-box گرفته نشود (که در غیر این صورت یک حلقهٔ مسیریابی ایجاد می‌کند).
   - **علامت فایروال (fwmark)** — یک SO_MARK اختیاری برای برچسب‌گذاری ترافیک هدایت‌شده (لینوکس)، منطبق با علامت واسط شما.
3. یک قانون مسیریابی اضافه کنید (یا یک قانون موجود را ویرایش کنید) و کنش آن را روی **واسط** بگذارید. اکنون دامنه‌های منطبق به واسط شما هدایت می‌شوند. اگر نام واسطی تنظیم نشده باشد، کنش به‌صورت ایمن به **پروکسی** تنزل می‌یابد.

> **نکتهٔ سکو:** روی **لینوکس** کاملاً پایدار است؛ اتصال روی ویندوز/مک هم کار می‌کند، اما مدیریت یک واسط با `table = off` در آن‌جا بر عهدهٔ خودتان است (در حد تلاش حداکثری).

## 🎁 هدیه: کانفیگ‌های رایگان Xray / sing-box

به‌عنوان یک هدیه کوچک به جامعه کاربران، IRBox یک **اشتراک عمومی رایگان** ارائه می‌دهد که با کلاینت‌های **Xray** و **sing-box** سازگار است.

🔗 **لینک اشتراک:**
```
https://raw.githubusercontent.com/frank-vpl/servers/refs/heads/main/irbox
```

## 📥 دانلود

اگر فقط می‌خواهید از IRBox استفاده کنید، یک نصاب از پیش ساخته‌شده برای سکوی خود را از **[صفحهٔ Releases](https://github.com/creatorofuniverses/IRBox/releases)** دریافت کنید — بدون نیاز به ابزار توسعه یا کامپایل:

| سکو | فایل‌ها |
|------|---------|
| **ویندوز** | `.exe` (نصاب NSIS) یا `.msi` |
| **مک** | `.dmg` (اینتل و Apple Silicon) |
| **لینوکس** | `.AppImage`، `.deb` یا `.rpm` |

> ℹ️ IRBox به‌طور پیش‌فرض در **حالت پروکسی** اجرا می‌شود (بدون نیاز به مجوز خاص). **حالت TUN** تمام ترافیک را مسیریابی می‌کند و به دسترسی بالا نیاز دارد — از مسیر **تنظیمات ← حالت VPN ← TUN ← اجرا به‌عنوان مدیر** استفاده کنید، یا برنامه را با `sudo` / به‌عنوان مدیر اجرا کنید.

## 🛠️ ساخت از منبع

برای توسعه، یا برای ساختن نصاب‌ها به‌دست خودتان.

### پیش‌نیازها
- **Rust و Cargo** (نسخهٔ پایدار)
- **Node.js و npm** (نسخهٔ ۱۸ به بالا)
- **Tauri CLI** — از وابستگیِ توسعهٔ پین‌شدهٔ `@tauri-apps/cli` تأمین می‌شود؛ دستور `npm install` (در پایین) آن را نصب می‌کند و با `npm run tauri` اجرا می‌شود. نیازی به `cargo install tauri-cli` جداگانه نیست — این کار نسخهٔ CLI را همراه با `@tauri-apps/api` قفل نگه می‌دارد.
- **وابستگی‌های سکو** ([پیش‌نیازهای Tauri](https://v2.tauri.app/start/prerequisites/)):
  - **لینوکس:** `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`
  - **ویندوز:** Microsoft C++ Build Tools به‌همراه WebView2 (روی ویندوز ۱۱ از پیش نصب است)
  - **مک:** Xcode Command Line Tools

### راه‌اندازی

1. **کلون کردن مخزن**
   ```bash
   git clone https://github.com/creatorofuniverses/IRBox.git
   cd IRBox
   ```

2. **نصب وابستگی‌های ظاهری**
   ```bash
   npm install
   ```

3. **دانلود هسته‌های پروکسی** (سایدکارهای sing-box و xray به‌همراه geoip/geosite). هدف (target) به‌طور خودکار از `rustc` تشخیص داده می‌شود؛ برای ساخت متقابل، هدف را صریح بدهید (مثلاً `./cores.sh x86_64-pc-windows-msvc`).

   **لینوکس/مک:**
   ```bash
   chmod +x cores.sh
   ./cores.sh
   ```

   **ویندوز:**
   ```bash
   ./cores.bat
   ```

### اجرا و ساخت

```bash
# اجرا در حالت توسعه (با بارگذاری مجدد خودکار)
npm run tauri dev

# ساخت نصاب‌های نسخهٔ نهایی برای سکوی فعلی
npm run tauri build
```

### نصب نسخه‌ای که ساختید

دستور `npm run tauri build` بسته‌های آمادهٔ نصب را در `src-tauri/target/release/bundle/` می‌نویسد. بستهٔ مربوط به سکوی خود را مستقیماً از همان‌جا نصب کنید:

- **لینوکس:**
  ```bash
  # AppImage — مستقیم اجرا می‌شود، نیازی به نصب نیست
  chmod +x src-tauri/target/release/bundle/appimage/IRBox_*.AppImage
  ./src-tauri/target/release/bundle/appimage/IRBox_*.AppImage

  # دبیان/اوبونتو
  sudo apt install ./src-tauri/target/release/bundle/deb/IRBox_*.deb

  # فدورا/RHEL
  sudo rpm -i src-tauri/target/release/bundle/rpm/IRBox-*.rpm
  ```
- **ویندوز:** نصاب `src-tauri\target\release\bundle\nsis\IRBox_*-setup.exe` (یا `msi\IRBox_*.msi`) را اجرا کنید.
- **مک:** فایل `src-tauri/target/release/bundle/dmg/IRBox_*.dmg` را باز کنید و **IRBox** را به پوشهٔ **Applications** بکشید.

> ساخت محلی دقیقاً همان نصاب‌هایی را تولید می‌کند که در [Releases](https://github.com/creatorofuniverses/IRBox/releases) منتشر می‌شوند — پس وقتی ساختید، به‌جای دانلود چیزی، از پوشهٔ `bundle/` نصب کنید.

## 📦 ساختن یک Release

‏Releaseها به‌طور خودکار توسط [ورک‌فلوی `Build`](.github/workflows/build.yaml) تولید می‌شوند که برای ویندوز (x86_64 و ARM64)، مک (اینتل و Apple Silicon) و لینوکس (x86_64) ساخته و سپس نصاب‌ها را در یک GitHub Release منتشر می‌کند. آن را به یکی از دو روش زیر اجرا کنید:

- **با push کردن یک تگ نسخه:**
  ```bash
  git tag v1.0.0
  git push origin v1.0.0
  ```
- **یا** اجرای دستی ورک‌فلو از تب **Actions** (*workflow_dispatch*) و وارد کردن نام تگ.

## 🤝 مشارکت

مشارکت‌ها خوش‌آمد هستند! لطفاً راحت باشید و یک درخواست کشش (Pull Request) ارسال کنید. برای تغییرات عمده، لطفاً ابتدا یک موضوع (issue) باز کنید تا در مورد آنچه می‌خواهید تغییر دهید، بحث کنیم.

## 📄 مجوز

این پروژه تحت مجوز عمومی گنو نسخه ۳.۰ (GPL-3.0) مجوز داده شده است - برای جزئیات بیشتر فایل [LICENSE](LICENSE) را ببینید.

### فناوری‌های هسته‌ای

اپلیکیشن IRBox از دو فناوری پیشرو در زمینه پروکسی استفاده می‌کند:

<div align="center">

| هسته | توضیحات |
|------|---------|
| [Xray-core](https://github.com/XTLS/Xray-core) | یک پلتفرم برای ساخت پروکسی‌های دور زدن محدودیت‌های شبکه |
| [sing-box](https://github.com/SagerNet/sing-box) | پلتفرم جهانی پروکسی |

</div>

### مجوزهای کتابخانه‌های شخص ثالث

- [Rust](https://www.rust-lang.org/) - [مجوز](./licenses/rust.md)
- [Tauri](https://v2.tauri.app/) - [مجوز](./licenses/tauri.md)
- [sing-box](https://github.com/SagerNet/sing-box) - [مجوز](./licenses/sing-box.md)
- [Xray-core](https://github.com/XTLS/Xray-core) - [مجوز](./licenses/xray.md)

## 🙏 قدردانی

- ساخته شده با [Tauri](https://tauri.app/) - چارچوبی برای ساخت برنامه‌های محلی امن
- قدرت گرفته از [sing-box](https://github.com/SagerNet/sing-box) و [Xray-core](https://github.com/XTLS/Xray-core)
- الهام گرفته از نیاز به راه‌حل‌های VPN امن و انعطاف‌پذیر

## 📚 مستندات
[مستندات IRBox](./docs/README.md)

## 🎨 دارایی‌های طراحی

<div align="center">

### لوگو و آیکون‌های برنامه
![PiraIcons](https://img.shields.io/badge/Icons_by-Hossein_Pira-3d85c6?style=for-the-badge&logo=github)

- آیکون‌ها توسط حسین پیرا – [PiraIcons](https://github.com/code3-dev/piraicons-assets) - [مجوز](./licenses/piraicons.md)

</div>

## 🧩 فناوری‌های مورد استفاده

<div align="center">

### وابستگی‌های ظاهری
![React](https://img.shields.io/badge/React-20232a?style=for-the-badge&logo=react&logoColor=61DAFB)
![TypeScript](https://img.shields.io/badge/TypeScript-007ACC?style=for-the-badge&logo=typescript&logoColor=white)
![Vite](https://img.shields.io/badge/Vite-B73BFE?style=for-the-badge&logo=vite&logoColor=FFD62E)

### چارچوب و هسته
![Tauri](https://img.shields.io/badge/Tauri-FFD62E?style=for-the-badge&logo=tauri&logoColor=black)
![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)

</div>

### وابستگی‌ها
- [react](https://react.dev/) - یک کتابخانه جاوا اسکریپت برای ساخت رابط‌های کاربری
- [react-dom](https://reactjs.org/docs/react-dom.html) - متدهای خاص DOM را فراهم می‌کند که می‌توانند در سطح بالای برنامه شما استفاده شوند
- [@tauri-apps/api](https://github.com/tauri-apps/tauri) - اتصالات API Tauri
- [@tauri-apps/plugin-deep-link](https://github.com/tauri-apps/plugins-workspace) - افزونه Tauri برای پیوند عمیق
- [@tauri-apps/plugin-shell](https://github.com/tauri-apps/plugins-workspace) - افزونه Tauri برای عملیات پوسته

#### وابستگی‌های توسعه
- [typescript](https://www.typescriptlang.org/) - تایپ‌اسکریپت یک زیرمجموعه تایپ‌دار از جاوا اسکریپت است که به جاوا اسکریپت ساده کامپایل می‌شود
- [vite](https://vitejs.dev/) - ابزارآلات ظاهری نسل بعدی
- [@vitejs/plugin-react](https://github.com/vitejs/vite-plugin-react) - افزونه Vite برای پروژه‌های React
- [@tauri-apps/cli](https://github.com/tauri-apps/tauri) - رابط خط فرمان Tauri
- [@types/react](https://www.npmjs.com/package/@types/react) - تعاریف تایپ برای React
- [@types/react-dom](https://www.npmjs.com/package/@types/react-dom) - تعاریف تایپ برای ReactDOM
