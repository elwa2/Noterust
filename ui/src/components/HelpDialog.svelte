<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import Modal from "./Modal.svelte";

  const dispatch = createEventDispatcher<{ close: void }>();

  const shortcuts: [string, string][] = [
    ["Ctrl+N", "ملاحظة جديدة"],
    ["Ctrl+O", "فتح ملف من القرص"],
    ["Ctrl+S / Ctrl+Shift+S", "حفظ / حفظ باسم"],
    ["Ctrl+W", "إغلاق التبويب"],
    ["Ctrl+Tab", "التبديل بين التبويبات"],
    ["Ctrl+F / Ctrl+H", "بحث / استبدال"],
    ["Ctrl+Shift+F", "بحث في كل الملاحظات"],
    ["Ctrl+P", "لوحة الأوامر"],
    ["Ctrl+G", "الذهاب إلى سطر"],
    ["Ctrl+D", "تكرار السطر"],
    ["F5", "إدراج التاريخ والوقت"],
    ["Ctrl+1 / 2 / 3 / 4", "الملاحظات / المهام / المالية / الأكواد والبرومبت"],
    ["Ctrl+B", "طيّ الشريط الجانبي إلى أيقونات / توسيعه"],
    ["Ctrl+Shift+N", "مجلد جديد"],
    ["Ctrl+Shift+T", "إبقاء النافذة فوق كل البرامج"],
    ["Ctrl+Shift+M", "وضع الملاحظة البسيطة (استيكر)"],
    ["Ctrl+,", "الإعدادات"],
    ["F1", "هذا الدليل"],
    ["Delete", "حذف إلى السلة"],
    ["/", "قائمة إدراج العناصر"],
    ["Ctrl + عجلة الفأرة", "تكبير/تصغير الخط"],
  ];
</script>

<Modal title="دليل استخدام NoteRust" wide on:close={() => dispatch("close")}>
  <div class="guide">
    <section>
      <h3>البدء</h3>
      <p>
        أنشئ ملاحظة بـ <kbd>Ctrl+N</kbd> واكتب مباشرة — يُحفظ كل شيء تلقائياً بعد نصف ثانية.
        المقاعد الثلاثة: الشريط الجانبي للمجلدات، قائمة الملاحظات، ثم المحرر.
      </p>
    </section>

    <section>
      <h3>المحرر</h3>
      <ul>
        <li>اكتب <code>/</code> في بداية سطر لإدراج عنوان أو قائمة أو قائمة تحقق أو كود أو جدول.</li>
        <li>حدّد نصاً واستخدم <strong>Aa</strong> للتنسيق بـ Markdown، أو <kbd>Ctrl+D</kbd> لتكرار السطر.</li>
        <li>قائمة التحقق <code>- [ ]</code> تظهر تلقائياً في صفحة المهام.</li>
        <li>زر الاتجاه يبدّل بين تلقائي/RTL/LTR، و«التفاف» يتحكم بأسطر النص الطويلة.</li>
        <li>
          الصق صورة من الحافظة (<kbd>Ctrl+V</kbd>) داخل الملاحظة فتُحفظ في مجلد الصور وتظهر
          في المعاينة (<kbd>Ctrl+E</kbd>) مع أزرار تكبير/تصغير، تدوير يميناً ويساراً، وإعادة.
        </li>
      </ul>
    </section>

    <section>
      <h3>الملفات</h3>
      <ul>
        <li>افتح أي ملف نصي بـ <kbd>Ctrl+O</kbd>؛ يُكتشف الترميز ونوع نهاية السطر تلقائياً.</li>
        <li>الملفات المفتوحة من القرص تُراقَب، ويظهر تنبيه إن تغيّرت من خارج التطبيق.</li>
        <li>«حفظ باسم» يكتب نسخة على القرص، و«تصدير» يحوّل الملاحظة إلى txt أو md.</li>
      </ul>
    </section>

    <section>
      <h3>التنظيم والخصوصية</h3>
      <ul>
        <li>اسحب الملاحظة إلى مجلد في الشريط الجانبي لنقلها، أو أفلتها على «كل الملاحظات».</li>
        <li>المحذوفات تبقى ٣٠ يوماً (قابلة للتغيير) ثم تُحذف نهائياً تلقائياً.</li>
        <li>قفل الملاحظة يشفّرها بـ AES-256-GCM؛ لا تُخزَّن كلمة المرور.</li>
        <li>تاريخ الإصدارات يحفظ نسخة قبل كل تعديل كبير ويمكن الاسترجاع منها.</li>
      </ul>
    </section>

    <section>
      <h3>المهام والمالية</h3>
      <ul>
        <li>المهام لها تاريخ استحقاق وأولوية وتكرار، وتُربط بملاحظة عند الحاجة.</li>
        <li>المالية تعمل بالجنيه المصري افتراضياً ويمكن تغيير العملة من الإعدادات؛
          المبالغ تُقبل بالأرقام العربية أو اللاتينية.</li>
      </ul>
    </section>

    <section class="keys">
      <h3>الاختصارات</h3>
      <ul>
        {#each shortcuts as [keys, label] (keys)}
          <li><kbd>{keys}</kbd><span>{label}</span></li>
        {/each}
      </ul>
    </section>
  </div>

  <svelte:fragment slot="footer">
    <button class="btn primary" on:click={() => dispatch("close")}>تم</button>
  </svelte:fragment>
</Modal>

<style>
  .guide {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px 22px;
    font-size: 0.86em;
  }

  section {
    break-inside: avoid;
  }

  h3 {
    margin: 6px 0 5px;
    font-size: 0.95em;
  }

  p,
  li {
    line-height: 1.8;
    color: var(--text-muted);
    margin: 0 0 4px;
  }

  ul {
    margin: 0;
    padding-inline-start: 18px;
  }

  code {
    background: var(--bg-raised);
    border-radius: 4px;
    padding: 0 4px;
    font-family: var(--font-family);
    color: var(--accent);
  }

  .keys ul {
    columns: 1;
    list-style: none;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .keys li {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-muted);
  }

  .keys kbd {
    min-width: 150px;
    text-align: center;
  }
</style>
