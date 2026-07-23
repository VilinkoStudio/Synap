package com.synap.app;

interface IShellService {
    int exec(in String[] command);
    void destroy();
}
