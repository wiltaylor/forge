import { Button, Modal } from '@forge/ui';

export default function ConfirmModal(props) {
  return (
    <Modal
      open={props.open}
      onClose={props.onCancel}
      title={props.title ?? 'Are you sure?'}
      footer={
        <div style={{ display: 'flex', gap: 'var(--sp-3)', 'justify-content': 'flex-end' }}>
          <Button onClick={props.onCancel}>Cancel</Button>
          <Button variant="danger" onClick={props.onConfirm}>
            {props.confirmLabel ?? 'Delete'}
          </Button>
        </div>
      }
    >
      {props.children}
    </Modal>
  );
}
