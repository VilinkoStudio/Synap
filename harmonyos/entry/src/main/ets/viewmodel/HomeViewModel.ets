// 主页ViewModel - 管理应用状态

import { Note } from '../models/NoteModel';
import { SynapService } from '../services/SynapService';

export class HomeViewModel {
  private synapService: SynapService;
  private notes: Note[] = [];
  private searchQuery: string = '';
  private isLoading: boolean = false;
  private error: string | null = null;
  
  // 观察者回调
  private onUpdate: ((notes: Note[]) => void) | null = null;
  private onLoadingChange: ((loading: boolean) => void) | null = null;
  private onError: ((error: string | null) => void) | null = null;
  
  constructor(synapService: SynapService) {
    this.synapService = synapService;
  }
  
  // 设置观察者
  setOnUpdate(callback: (notes: Note[]) => void): void {
    this.onUpdate = callback;
  }
  
  setOnLoadingChange(callback: (loading: boolean) => void): void {
    this.onLoadingChange = callback;
  }
  
  setOnError(callback: (error: string | null) => void): void {
    this.onError = callback;
  }
  
  // 加载笔记
  async loadNotes(): Promise<void> {
    this.setLoading(true);
    this.setError(null);
    
    try {
      this.notes = await this.synapService.getRecentNotes();
      this.notifyUpdate();
    } catch (error) {
      this.setError(`加载失败: ${String(error)}`);
    } finally {
      this.setLoading(false);
    }
  }
  
  // 搜索笔记
  async searchNotes(query: string): Promise<void> {
    this.searchQuery = query;
    
    if (query.length === 0) {
      await this.loadNotes();
      return;
    }
    
    this.setLoading(true);
    this.setError(null);
    
    try {
      this.notes = await this.synapService.search(query);
      this.notifyUpdate();
    } catch (error) {
      this.setError(`搜索失败: ${String(error)}`);
    } finally {
      this.setLoading(false);
    }
  }
  
  // 创建笔记
  async createNote(content: string, tags: string[] = []): Promise<Note> {
    this.setLoading(true);
    this.setError(null);
    
    try {
      const note = await this.synapService.createNote(content, tags);
      await this.loadNotes(); // 刷新列表
      return note;
    } catch (error) {
      this.setError(`创建失败: ${String(error)}`);
      // 必须抛出 Error 对象
      if (error instanceof Error) {
        throw error;
      } else {
        throw new Error(String(error));
      }
    } finally {
      this.setLoading(false);
    }
  }
  
  // 删除笔记
  async deleteNote(noteId: string): Promise<void> {
    this.setLoading(true);
    this.setError(null);
    
    try {
      await this.synapService.deleteNote(noteId);
      await this.loadNotes(); // 刷新列表
    } catch (error) {
      this.setError(`删除失败: ${String(error)}`);
    } finally {
      this.setLoading(false);
    }
  }
  
  // 获取当前笔记列表
  getNotes(): Note[] {
    return this.notes;
  }
  
  // 获取加载状态
  getIsLoading(): boolean {
    return this.isLoading;
  }
  
  // 获取错误信息
  getError(): string | null {
    return this.error;
  }
  
  // 设置加载状态
  private setLoading(loading: boolean): void {
    this.isLoading = loading;
    if (this.onLoadingChange) {
      this.onLoadingChange(loading);
    }
  }
  
  // 设置错误信息
  private setError(error: string | null): void {
    this.error = error;
    if (this.onError) {
      this.onError(error);
    }
  }
  
  // 通知更新
  private notifyUpdate(): void {
    if (this.onUpdate) {
      this.onUpdate(this.notes);
    }
  }
}