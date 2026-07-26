export function createItemEditController({
  clipboardApi,
  getEditContent,
  getAnnotationDraft,
  setEditingId,
  setEditContent,
  setAnnotationEditingId,
  setAnnotationDraft,
  showActionError,
  showActionNotice,
  updateVisibleItem,
}) {
  function startContentEdit(item) {
    if (item.type !== 'text') return;
    setEditingId(item.id);
    setEditContent(item.content || '');
    setAnnotationEditingId(null);
    setAnnotationDraft('');
  }

  function cancelContentEdit() {
    setEditingId(null);
    setEditContent('');
  }

  function startAnnotationEdit(item) {
    setAnnotationEditingId(item.id);
    setAnnotationDraft(item.annotation || '');
    setEditingId(null);
    setEditContent('');
  }

  function cancelAnnotationEdit() {
    setAnnotationEditingId(null);
    setAnnotationDraft('');
  }

  async function saveContentEdit(itemId) {
    const editContent = getEditContent();
    if (!editContent.trim()) {
      showActionError('原文不能为空');
      return;
    }

    try {
      const updatedItem = await clipboardApi.updateItemContent(itemId, editContent);
      updateVisibleItem(itemId, () => updatedItem);

      setEditingId(null);
      setEditContent('');
      showActionNotice('原文已更新');
    } catch (e) {
      console.error('保存原文失败:', e);
      showActionError('保存原文失败: ' + e);
    }
  }

  async function saveAnnotation(itemId) {
    try {
      const savedAnnotation = await clipboardApi.updateItemAnnotation(itemId, getAnnotationDraft());

      updateVisibleItem(itemId, (item) => ({
        ...item,
        annotation: savedAnnotation,
      }));

      setAnnotationEditingId(null);
      setAnnotationDraft('');
      showActionNotice(savedAnnotation ? '标注已保存' : '标注已清除');
    } catch (e) {
      console.error('保存标注失败:', e);
      showActionError('保存标注失败: ' + e);
    }
  }

  return {
    cancelAnnotationEdit,
    cancelContentEdit,
    saveAnnotation,
    saveContentEdit,
    startAnnotationEdit,
    startContentEdit,
  };
}
