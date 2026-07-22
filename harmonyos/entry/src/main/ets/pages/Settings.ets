// 设置页面 - 使用 SymbolGlyph

import router from '@ohos.router';

@Entry
@Component
struct Settings {
  @State slideInX: number = 100;
  @State opacityValue: number = 0;

  aboutToAppear(): void {
    this.slideInX = 100;
    this.opacityValue = 0;

    setTimeout(() => {
      this.slideInX = 0;
      this.opacityValue = 1;
    }, 100);
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

          Text('设置')
            .fontSize(18)
            .fontWeight(FontWeight.Bold)
            .fontColor('#FFFFFF')

          Blank()
        }
        .width('100%')
        .padding({ left: 16, right: 16 })
      }

      Scroll() {
        Column({ space: 16 }) {
          Column() {
            Row() {
              Column() {
                SymbolGlyph($r('sys.symbol.person_circle_fill'))
                  .fontSize(48)
                  .fontColor(['#4CAF50'])
                  .width(56)
                  .height(56)
              }
              .margin({ right: 16 })

              Column() {
                Text('Synap')
                  .fontSize(18)
                  .fontWeight(FontWeight.Bold)
                  .fontColor('#FFFFFF')

                Text('极简思维捕捉应用')
                  .fontSize(14)
                  .fontColor('#888888')
                  .margin({ top: 4 })

                Text('版本 1.0.0')
                  .fontSize(12)
                  .fontColor('#666666')
                  .margin({ top: 4 })
              }
            }
            .width('100%')
            .padding(20)
          }
          .width('92%')
          .backgroundColor('#2B2B2B')
          .borderRadius(16)
          .margin({ top: 20 })

          Text('功能设置')
            .fontSize(14)
            .fontColor('#888888')
            .padding({ left: 24 })
            .margin({ top: 16 })

          SettingsItem({
            icon: $r('sys.symbol.bell_fill'),
            title: '通知设置',
            subtitle: '管理应用通知',
            hasArrow: true
          })

          SettingsItem({
            icon: $r('sys.symbol.paintbrush_fill'),
            title: '主题设置',
            subtitle: '深色模式',
            hasArrow: true
          })

          SettingsItem({
            icon: $r('sys.symbol.globe'),
            title: '语言设置',
            subtitle: '简体中文',
            hasArrow: true
          })

          Text('数据管理')
            .fontSize(14)
            .fontColor('#888888')
            .padding({ left: 24 })
            .margin({ top: 24 })

          SettingsItem({
            icon: $r('sys.symbol.arrow_down_doc_fill'),
            title: '导入数据',
            subtitle: '从其他应用导入',
            hasArrow: true
          })

          SettingsItem({
            icon: $r('sys.symbol.arrow_up_doc_fill'),
            title: '导出数据',
            subtitle: '备份笔记数据',
            hasArrow: true
          })

          SettingsItem({
            icon: $r('sys.symbol.trash_fill'),
            title: '清除缓存',
            subtitle: '释放存储空间',
            hasArrow: true
          })

          Text('关于')
            .fontSize(14)
            .fontColor('#888888')
            .padding({ left: 24 })
            .margin({ top: 24 })

          SettingsItem({
            icon: $r('sys.symbol.questionmark_circle_fill'),
            title: '帮助与反馈',
            subtitle: '获取使用帮助',
            hasArrow: true
          })

          SettingsItem({
            icon: $r('sys.symbol.lock_fill'),
            title: '隐私政策',
            subtitle: '查看隐私条款',
            hasArrow: true
          })

          SettingsItem({
            icon: $r('sys.symbol.doc_text_fill'),
            title: '用户协议',
            subtitle: '查看服务条款',
            hasArrow: true
          })

          Column() {
            Text('Synap 鸿蒙版')
              .fontSize(14)
              .fontColor('#666666')

            Text('Version 1.0.0')
              .fontSize(12)
              .fontColor('#666666')
              .margin({ top: 4 })
          }
          .width('100%')
          .padding(20)
          .margin({ top: 24, bottom: 40 })
          .justifyContent(FlexAlign.Center)
        }
        .width('100%')
      }
      .width('100%')
      .layoutWeight(1)
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

@Component
struct SettingsItem {
  @Prop icon: Resource;
  @Prop title: string;
  @Prop subtitle: string;
  @Prop hasArrow: boolean = false;

  @State isPressed: boolean = false;

  build() {
    Row() {
      SymbolGlyph(this.icon)
        .fontSize(24)
        .fontColor(['#4CAF50'])
        .margin({ right: 16 })

      Column({ space: 4 }) {
        Text(this.title)
          .fontSize(16)
          .fontColor('#FFFFFF')

        Text(this.subtitle)
          .fontSize(13)
          .fontColor('#888888')
      }
      .layoutWeight(1)

      if (this.hasArrow) {
        SymbolGlyph($r('sys.symbol.chevron_right'))
          .fontSize(16)
          .fontColor(['#888888'])
      }
    }
    .width('92%')
    .height(72)
    .padding({ left: 20, right: 20 })
    .backgroundColor('#2B2B2B')
    .borderRadius(12)
    .scale({ x: this.isPressed ? 0.98 : 1, y: this.isPressed ? 0.98 : 1 })
    .animation({
      duration: 100,
      curve: Curve.EaseOut
    })
    .onTouch((event: TouchEvent): void => {
      if (event.type === TouchType.Down) {
        this.isPressed = true;
      } else if (event.type === TouchType.Up || event.type === TouchType.Cancel) {
        this.isPressed = false;
      }
    })
  }
}