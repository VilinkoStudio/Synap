declare global {
  namespace App {
    interface Error {
      message: string;
      code?: string;
    }
  }
}

export {};
