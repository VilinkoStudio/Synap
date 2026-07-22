// 笔记详情页（符合鸿蒙设计规范）

import router from '@ohos.router';
import { Note } from '../models/NoteModel';
import { Utils } from '../common/Utils';
import { SynapService } from '../services/SynapService';

@Entry
@Component
struct NoteDetail {
  @State note: Note | null = null;
  @State isLoading: boolean = true;
  @State showEditMode: boolean = false;
  @State editContent: string = '';
  @State editTags: string = '';

  @State slideInX: number = 100;
  @State opacityValue: number = 0;

  aboutToAppear(): void {
    this.slideInX = 100;
    this.opacityValue = 0;

    setTimeout(() => {
      this.slideInX = 0;
      this.opacityValue = 1;
    }, 100);

    const params = router.getParams();
    const noteId = params?.noteId as string;

    if (noteId) {
      this.loadNote(noteId);
    }
  }

  loadNote(noteId: string): void {
    this.isLoading = true;

    let synapService = AppStorage.get<SynapService>('synapService');
    if (!synapService) {
      this.isLoading = false;
      return;
    }

    synapService.getNote(noteId).then((note: Note | null) => {
      this.note = note;
      if (this.note) {
        this.editContent = this.note.content;
        this.editTags = this.note.tags.join(', ');
      }
    }).catch((err: Error) => {
      console.error('加载笔记失败:', err.message);
    }).finally(() => {
      this.isLoading = false;
    });
  }

  saveEdit(): void {
    if (!this.note) return;

    let synapService = AppStorage.get<SynapService>('synapService');
    if (!synapService) return;

    const tags = Utils.parseTags(this.editTags);

    synapService.editNote(this.note.id, this.editContent, tags).then(() => {
      this.showEditMode = false;
      if (this.note) {
        this.loadNote(this.note.id);
      }
    }).catch((err: Error) => {
      console.error('保存失败:', err.message);
    });
  }

  deleteNote(): void {
    if (!this.note) return;

    let synapService = AppStorage.get<SynapService>('synapService');
    if (!synapService) return;

    synapService.deleteNote(this.note.id).then(() => {
      router.back();
    }).catch((err: Error) => {
      console.error('删除失败:', err.message);
    });
  }

  build() {
    Column() {
      Stack({ alignContent: Alignment.Center }) {
        Row()
          .width('100%')
          .height(80)
          .backgroundColor('#2B2B2B')
          .linearGradient({
            direction: GradientDirection.Bottom,
            colors: [['#3A3A3A', 1], ['#2B2B2B', 0]]
          })

        Row() {
          Column() {
            SymbolGlyph($r('sys.symbol.chevron_left'))
              .fontSize(24)
              .fontColor(['#FFFFFF'])
          }
          .width(48)
          .height(48)
          .justifyContent(FlexAlign.Center)
          .onClick(() => {
            router.back();
          })

          Text('笔记详情')
            .fontSize(18)
            .fontWeight(FontWeight.Bold)
            .fontColor('#FFFFFF')

          Blank()

          Column() {
            SymbolGlyph($r('sys.symbol.pencil'))
              .fontSize(22)
              .fontColor(['#FFFFFF'])
          }
          .width(48)
          .height(48)
          .justifyContent(FlexAlign.Center)
          .onClick(() => {
            this.showEditMode = true;
          })

          Column() {
            SymbolGlyph($r('sys.symbol.trash'))
              .fontSize(22)
              .fontColor(['#FF6B6B'])
          }
          .width(48)
          .height(48)
          .justifyContent(FlexAlign.Center)
          .onClick(() => {
            this.deleteNote();
          })
        }
        .width('100%')
        .padding({ left: 16, right: 16 })
      }

      if (this.isLoading) {
        Column() {
          LoadingProgress()
            .width(50)
            .height(50)
            .color('#4CAF50')
        }
        .width('100%')
        .layoutWeight(1)
        .justifyContent(FlexAlign.Center)
      } else if (!this.note) {
        Column() {
          Text('笔记不存在')
            .fontSize(18)
            .fontColor('#888888')

          Button('返回')
            .fontSize(16)
            .fontColor('#FFFFFF')
            .backgroundColor('#4CAF50')
            .margin({ top: 16 })
            .onClick(() => {
              router.back();
            })
        }
        .width('100%')
        .layoutWeight(1)
        .justifyContent(FlexAlign.Center)
      } else {
        Scroll() {
          Column() {
            if (this.showEditMode) {
              TextArea({
                placeholder: '输入笔记内容...',
                text: this.editContent
              })
              .width('100%')
              .height(300)
              .backgroundColor('#2B2B2B')
              .borderRadius(12)
              .margin({ bottom: 16 })
              .padding(16)
              .fontColor('#FFFFFF')
              .placeholderColor('#666666')
              .onChange((value: string) => {
                this.editContent = value;
              })

              Row() {
                SymbolGlyph($r('sys.symbol.tag'))
                  .fontSize(20)
                  .fontColor(['#4CAF50'])
                  .margin({ right: 8 })

                TextInput({
                  placeholder: '标签（逗号分隔）',
                  text: this.editTags
                })
                .width('100%')
                .height(44)
                .backgroundColor('#2B2B2B')
                .borderRadius(8)
                .fontColor('#FFFFFF')
                .placeholderColor('#666666')
                .onChange((value: string) => {
                  this.editTags = value;
                })
              }
              .width('100%')

              Row({ space: 12 }) {
                Button('取消')
                  .width('45%')
                  .height(48)
                  .backgroundColor('#3A3A3A')
                  .fontSize(16)
                  .fontColor('#FFFFFF')
                  .borderRadius(12)
                  .margin({ top: 16 })
                  .onClick(() => {
                    this.showEditMode = false;
                    this.editContent = this.note?.content || '';
                    this.editTags = this.note?.tags.join(', ') || '';
                  })

                Button('保存')
                  .width('45%')
                  .height(48)
                  .backgroundColor('#4CAF50')
                  .fontSize(16)
                  .fontColor('#FFFFFF')
                  .borderRadius(12)
                  .margin({ top: 16 })
                  .onClick(() => {
                    this.saveEdit();
                  })
              }
              .width('100%')
            } else {
              Text(this.note.content)
                .fontSize(18)
                .fontColor('#FFFFFF')
                .lineHeight(28)
                .textAlign(TextAlign.Start)
                .margin({ bottom: 24 })

              if (this.note.tags.length > 0) {
                Text('标签')
                  .fontSize(14)
                  .fontColor('#888888')
                  .margin({ bottom: 8 })

                Flex({ wrap: FlexWrap.Wrap }) {
                  ForEach(this.note.tags, (tag: string) => {
                    Text(`#${tag}`)
                      .fontSize(14)
                      .fontColor('#4CAF50')
                      .backgroundColor('#1E1E1E')
                      .borderRadius(6)
                      .padding({ left: 10, right: 10, top: 5, bottom: 5 })
                      .margin({ right: 10, bottom: 8 })
                  }, (tag: string) => tag)
                }
              }

              Column() {
                Text('创建时间')
                  .fontSize(14)
                  .fontColor('#888888')
                  .margin({ bottom: 4 })

                Text(Utils.formatTime(this.note.createdAt, 'YYYY-MM-DD HH:mm:ss'))
                  .fontSize(14)
                  .fontColor('#FFFFFF')
              }
              .margin({ top: 24 })

              Column() {
                Text('笔记ID')
                  .fontSize(12)
                  .fontColor('#666666')
                  .margin({ bottom: 4 })

                Text(this.note.id)
                  .fontSize(12)
                  .fontColor('#666666')
                  .fontFamily('monospace')
              }
              .margin({ top: 16 })
            }
          }
          .width('100%')
          .padding(24)
        }
        .width('100%')
        .layoutWeight(1)
      }
    }
    .width('100%')
    .height('100%')
    .backgroundColor('#1E1E1E')
    .translate({ x: this.slideInX })
    .opacity(this.opacityValue)
    .animation({
      duration: 300,
      curve: Curve.EaseOut
    })
  }
}