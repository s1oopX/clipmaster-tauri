import flatpickr from 'flatpickr';
import { Mandarin } from 'flatpickr/dist/l10n/zh.js';
import 'flatpickr/dist/flatpickr.css';

import { formatDateKey } from './clipboard-ui.js';

export function createDatePickerAction(onSelectDay) {
  return function datePicker(node, params) {
    let availableDateKeys = new Set(params.availableDays.map((day) => day.date_key));
    let suppressChange = false;

    const picker = flatpickr(node, {
      allowInput: false,
      ariaDateFormat: 'Y年m月d日',
      clickOpens: true,
      dateFormat: 'Y-m-d',
      defaultDate: params.selectedDay || undefined,
      disableMobile: true,
      locale: Mandarin,
      monthSelectorType: 'static',
      nextArrow: '<span class="date-nav-label">下月</span>',
      prevArrow: '<span class="date-nav-label">上月</span>',
      shorthandCurrentMonth: false,
      onChange: (_selectedDates, dateStr) => {
        if (suppressChange) return;
        void onSelectDay(dateStr);
      },
      onDayCreate: (_dObj, _dStr, _fp, dayElem) => {
        if (availableDateKeys.has(formatDateKey(dayElem.dateObj))) {
          dayElem.classList.add('has-clipboard-items');
        }
      },
    });

    picker.calendarContainer.classList.add('clipmaster-date-picker');
    picker.prevMonthNav?.setAttribute('aria-label', '上个月');
    picker.nextMonthNav?.setAttribute('aria-label', '下个月');

    function sync(nextParams) {
      availableDateKeys = new Set(nextParams.availableDays.map((day) => day.date_key));

      suppressChange = true;
      try {
        if (nextParams.selectedDay) {
          picker.setDate(nextParams.selectedDay, false);
        } else {
          picker.clear(false);
        }

        picker.redraw();
      } finally {
        queueMicrotask(() => {
          suppressChange = false;
        });
      }
    }

    return {
      update: sync,
      destroy() {
        picker.destroy();
      },
    };
  };
}
