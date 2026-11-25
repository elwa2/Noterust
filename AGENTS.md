# NoteRust — تعليمات المساعد

## قواعد صارمة: حماية مساحة القرص (إلزامي)

المشروع صغير، لكن بناء Rust/Tauri بياكل أكتر من 10 جيجا على جهاز المستخدم. ممنوع منعًا باتًا تنتهك القواعد دي، حتى لو بدا إن ده أسهل أو أسرع.

### ممنوع
- ممنوع تشغيل `cargo build` أو `cargo run` أو `cargo tauri build` أو `cargo tauri dev` أو `npm run tauri ...` محليًا.
- ممنوع تثبيت أي أداة أو toolchain أو حزمة عالمية (rustup components، Visual Studio Build Tools، Node packages عالمية، إلخ).
- ممنوع أي أمر بينزّل dependencies من النت (cargo fetch / cargo update / npm install). الـ dependencies موجودة جاهزة في فولدر `vendor`، فاستخدم `--offline` أو `--frozen` دايمًا.
- ممنوع تحط فولدر `target` جوه المشروع.
- ممنوع تعدّل `Cargo.lock` أو `rust-toolchain.toml` أو فولدر `vendor` إلا لو المستخدم طلب ده صراحة.

### المسموح للتحقق من الكود (فقط)
- `cargo check --offline` بس، ومعاه target خارج المشروع في مكان مؤقت:
  - Windows (PowerShell): `$env:CARGO_TARGET_DIR="$env:TEMP\noterust-target"`
  - بعد ما تخلص: امسح الفولدر ده بالكامل (`Remove-Item -Recurse -Force $env:TEMP\noterust-target`).
- لو تعديل الواجهة بس (فولدر `ui`)، متشغّلش أي أمر Rust خالص. عدّل الملفات وبس.

### قاعدة البناء
- البناء الفعلي بيتم على GitHub Actions (فولدر `.github`) مش على الجهاز.
- لما تخلص تعديلات: لخّصلي إيه اللي اتغير، وقوللي أعمل commit وpush ويبني الـ workflow.
- لو مفيش workflow بيبني التطبيق، اقترح تعمله بدل ما تبني محليًا.

### لو محتاج حاجة تخالف القواعد دي
وقّف واسأل المستخدم الأول واشرحله ليه ومساحة كام هتاخد. متنفّذش من نفسك.
