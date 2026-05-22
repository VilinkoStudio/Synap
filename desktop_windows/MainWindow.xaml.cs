using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices.WindowsRuntime;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Data;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Navigation;
using Windows.Foundation;
using Windows.Foundation.Collections;
using WinRT.Interop;
using Microsoft.UI.Windowing;
using Microsoft.UI;

// To learn more about WinUI, the WinUI project structure,
// and more about our project templates, see: http://aka.ms/winui-project-info.

namespace desktop_windows
{
    /// <summary>
    /// An empty window that can be used on its own or navigated to within a Frame.
    /// </summary>
    public sealed partial class MainWindow : Window
    {
        public MainWindow()
        {
            InitializeComponent();
            SetupTitleBar();
            contentFrame.Navigate(typeof(ListPage));
            SamplePage1Item.IsSelected = true;
        }

        private void SetupTitleBar()
        {
            var titleBar = AppWindow.TitleBar;
            titleBar.IconShowOptions = IconShowOptions.HideIconAndSystemMenu;
            titleBar.ExtendsContentIntoTitleBar = true;
            
            AppTitleBar.Loaded += AppTitleBar_Loaded;
            AppTitleBar.SizeChanged += AppTitleBar_SizeChanged;
            
            ((FrameworkElement)Content).ActualThemeChanged += MainWindow_ActualThemeChanged;

            nvSample.Loaded += NavigationView_Loaded;
        }

        private void MainWindow_ActualThemeChanged(FrameworkElement sender, object args)
        {
            UpdateTitleBarColors();
        }

        private void AppTitleBar_Loaded(object sender, RoutedEventArgs e)
        {
            UpdateTitleBarColors();
        }

        private void AppTitleBar_SizeChanged(object sender, SizeChangedEventArgs e)
        {
            UpdateTitleBarColors();
        }

        private void UpdateTitleBarColors()
        {
            var titleBar = AppWindow.TitleBar;
            var isDark = ((FrameworkElement)Content).ActualTheme == ElementTheme.Dark;
            
            titleBar.ButtonBackgroundColor = Colors.Transparent;
            titleBar.ButtonForegroundColor = isDark ? Colors.White : Colors.Black;
            titleBar.ButtonHoverBackgroundColor = isDark ? Windows.UI.Color.FromArgb(255, 50, 50, 50) : Windows.UI.Color.FromArgb(255, 230, 230, 230);
            titleBar.ButtonPressedBackgroundColor = isDark ? Windows.UI.Color.FromArgb(255, 70, 70, 70) : Windows.UI.Color.FromArgb(255, 200, 200, 200);
            
            titleBar.BackgroundColor = Colors.Transparent;
            titleBar.ForegroundColor = isDark ? Colors.White : Colors.Black;
        }

        private void NavigationView_Loaded(object sender, RoutedEventArgs e)
        {
            var settingsItem = nvSample.SettingsItem as NavigationViewItem;
            if (settingsItem != null)
            {
                settingsItem.Content = "设置";
            }
        }

        private void BackButton_Click(object sender, RoutedEventArgs e)
        {
            if (contentFrame.CanGoBack)
            {
                contentFrame.GoBack();
            }
        }

        private void NavigationView_SelectionChanged(NavigationView sender, NavigationViewSelectionChangedEventArgs args)
        {
            if (args.IsSettingsSelected)
            {
                contentFrame.Navigate(typeof(SettingsPage));
                nvSample.Header = "设置";
            }
            else if (args.SelectedItem is NavigationViewItem selectedItem)
            {
                switch (selectedItem.Tag.ToString())
                {
                    case "NewNote":
                        contentFrame.Navigate(typeof(ListPage));
                        nvSample.Header = "新增笔记";
                        break;
                    case "ListPage":
                        contentFrame.Navigate(typeof(ListPage));
                        nvSample.Header = "笔记列表";
                        break;
                    case "SamplePage2":
                        contentFrame.Navigate(typeof(TrashPage));
                        nvSample.Header = "回收站";
                        break;
                }
            }
            BackButton.Visibility = contentFrame.CanGoBack ? Visibility.Visible : Visibility.Collapsed;
        }
    }
}
