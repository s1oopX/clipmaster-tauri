import { convertFileSrc } from '@tauri-apps/api/core';
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';

const container = document.getElementById('container');
const image = document.getElementById('image');
const closeBtn = document.getElementById('close-btn');
const resizeHandles = document.querySelectorAll('.resize-handle');
const currentWin = getCurrentWindow();
const MIN_PIN_SIZE = 100;
const MAX_PIN_WIDTH = 720;
const MAX_PIN_HEIGHT = 520;

async function runWindowAction(action, label) {
  try {
    await action();
  } catch (error) {
    console.error(`${label}失败:`, error);
  }
}

const urlParams = new URLSearchParams(window.location.search);
const imagePath = urlParams.get('path');

if (imagePath) {
  image.src = convertFileSrc(imagePath);
} else {
  console.error('未提供图片路径');
}

function fitImageWindowSize(naturalWidth, naturalHeight) {
  const imageWidth = Math.max(1, naturalWidth);
  const imageHeight = Math.max(1, naturalHeight);
  const ratio = Math.min(MAX_PIN_WIDTH / imageWidth, MAX_PIN_HEIGHT / imageHeight, 1);

  return {
    width: Math.max(MIN_PIN_SIZE, Math.round(imageWidth * ratio)),
    height: Math.max(MIN_PIN_SIZE, Math.round(imageHeight * ratio)),
  };
}

image.onload = () => {
  const { width, height } = fitImageWindowSize(image.naturalWidth, image.naturalHeight);

  runWindowAction(
    () => currentWin.setSize(new LogicalSize(width, height)),
    '调整贴图窗口大小'
  );
};

image.onerror = (event) => {
  console.error('图片加载失败:', event);
  image.alt = '图片加载失败';
};

async function resizePinnedWindow(delta) {
  const nextWidth = Math.max(100, Math.min(Math.round(window.innerWidth * delta), 2400));
  const nextHeight = Math.max(100, Math.min(Math.round(window.innerHeight * delta), 2400));

  await runWindowAction(
    () => currentWin.setSize(new LogicalSize(nextWidth, nextHeight)),
    '同步缩放贴图窗口'
  );
}

container.addEventListener(
  'wheel',
  async (event) => {
    if (event.ctrlKey) {
      event.preventDefault();

      const delta = event.deltaY < 0 ? 1.1 : 0.9;
      await resizePinnedWindow(delta);
    }
  },
  { passive: false }
);

container.addEventListener('pointerdown', async (event) => {
  if (event.button !== 0) return;
  if (event.target.closest('button')) return;

  await runWindowAction(() => currentWin.startDragging(), '移动贴图');
});

resizeHandles.forEach((handle) => {
  handle.addEventListener('pointerdown', async (event) => {
    if (event.button !== 0) return;

    event.preventDefault();
    event.stopPropagation();
    await runWindowAction(
      () => currentWin.startResizeDragging(handle.dataset.direction),
      '缩放贴图窗口'
    );
  });
});

closeBtn.addEventListener('click', async (event) => {
  event.stopPropagation();
  await runWindowAction(() => currentWin.close(), '关闭贴图');
});

document.addEventListener('keydown', async (event) => {
  if (event.key === 'Escape') {
    await runWindowAction(() => currentWin.close(), '关闭贴图');
  }
});
