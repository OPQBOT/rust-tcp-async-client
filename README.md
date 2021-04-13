# rust-tcp-async-client

rust实现的异步多客户端网络框架，基于[tokio](https://github.com/tokio-rs/tokio)和[mlua](https://github.com/khvzak/mlua),可自定义通讯协议
插件化采用lua。应用场景im,game server,bot等.golang 实现的网络框架就很多了,个人希望能在rust做一些微不足道的贡献,该项目的代码也很简陋属于 `demo `,线上生产还是需要深加工.欢迎👏各位大佬吐槽毕竟我还是`rust萌新`项目更多的代码 Copy 自 [tox-rs](https://github.com/tox-rs/tox) hhhhhh

## 🎈框架🎈
主线程维护多个client,将消息分发至lua。

## 🎈性能🎈
性能和并发这我不想说,我等萌新再弱鸡,Rust的优势会弥补我们的不足。弘扬Rust势在必行emm......Golang弟弟表示不服....

## 🎈协议🎈
自定义协议部分并没有抽离出来，因为本人正处于并将长期处于萌新阶段。。hhhh 请阅读源码`codec.rs`来实现自己的协议即可 编码器采用的 `tokio`的`Codec`  

## 🎈插件🎈

在Plugins目录下已给出demo 默认绑定了2个函数 `OnChatMsg` 和 `OnChatEvent` 收到消息的时候会遍历插件并调用`OnChatMsg`和收到相关事件的时候会遍历插件并调用`OnChatEvent` demo中绑定了3个luaApi 详情请见`test.lua`  

## 🎈指南食用🎈

1⃣️ 克隆项目
```bash
git clone https://github.com/OPQBOT/rust-tcp-async-client.git
```
2⃣️

```bash
cd rust-tcp-async-client

```

3⃣️ VSCode打开

```bash
code .
```

4⃣️ 启动server

```bash
cd examples
cargo run --package examples --example server-test
```

5⃣ 启动client

```bash
cd examples
cargo run --package examples --example client-test
```

6⃣ Coding YourSelf

## 🎈交流🎈


<img src="https://camo.githubusercontent.com/93f9b87a271da3b096ebdcd679dac0336531f0281e54c1172f7b965a6f34c6d8/68747470733a2f2f7a332e617831782e636f6d2f323032312f30342f31332f6373685648302e6a7067" alt="Drawing" width="180px" />  <img src="https://camo.githubusercontent.com/b470ea479c9676cf02bafa549171bde339bb9e415507daf5ef3fcbe7edd99c72/68747470733a2f2f7a332e617831782e636f6d2f323032312f30342f31332f6373686545562e6a7067" alt="Drawing" width="180px" />

## 🎈License🎈

Licensed under [GPLv3+](/LICENSE) .
