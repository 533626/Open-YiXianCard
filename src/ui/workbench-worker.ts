import { handleWorkbenchWorkerRequest } from "./worker-handler";
import type {
  WorkbenchWorkerRequest,
  WorkbenchWorkerResponse,
} from "./worker-protocol";

interface WorkbenchWorkerScope {
  onmessage: ((event: MessageEvent<WorkbenchWorkerRequest>) => void) | null;
  postMessage(message: WorkbenchWorkerResponse): void;
  __openYiXianWorkerMarker?: string;
}

const workerScope = globalThis as unknown as WorkbenchWorkerScope;
workerScope.__openYiXianWorkerMarker = "open-yixiancard/workbench-worker";

workerScope.onmessage = (event) => {
  void handleWorkbenchWorkerRequest(event.data).then((response) => {
    workerScope.postMessage(response);
  });
};
