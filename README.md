为“复杂网络分析”课程设计一个验证 `crates` 依赖网络（`crates.io`）是否具备“无标度”和“小世界”等特征的课题，在理论意义、数据获取和实现难度上都是一个**绝佳的选择**。

在学术界，这已经不是一个猜测，而是有明确结论的方向。对此类软件生态系统的网络分析通常能证实以下结论：

* **无标度 (Scale-Free) 特征**：依赖网络中存在少数枢纽节点（`serde`、`rand` 等核心库），它们被大量其他包所依赖。这个特性已被学者通过研究 `crates.io` 等仓库证实。
* **小世界 (Small-World) 特征**：从任意一个包出发，通常只需很少的几步就能通过依赖关系到达另一个不相关的包。这个特征在许多语言生态的包依赖网络（如 `npm`、`crates.io` 等）中被发现。
* **模块化 (Modularity) 结构**：依赖网络中存在紧密连接的社区结构，这印证了软件工程的高内聚原则。

以下从技术实现层面，提供一个清晰的课程设计路线作为参考。

---

### 项目设计与实现路线

本项目主要由四个清晰的阶段构成，非常适合作为课程设计或毕业设计的课题。

#### 阶段一：获取`crates.io`依赖关系数据

要构建分析网络，首要任务是获取所有 crate 之间的依赖关系数据。可以主要参考以下方法构建自己的数据集：

1. **使用数据库快照**
    `crates.io` 官方会发布每日的数据库快照，这是构建完整依赖网络图最全面的数据源。
    * **数据源**：我已通过 `wget https://static.crates.io/db-dump.tar.gz` 直接下载。

2. **使用`crates.io`索引**
    我在工作区 `git clone https://github.com/rust-lang/crates.io-index` 仓库，它的数据完全足够构建出一个高质量的元数据依赖网络。每个文件包含crate的元数据，示例如下：

    ```json
    {"name":"deepseek-sdk","vers":"0.2.0","deps":[{"name":"derive_builder","req":"^0.20.2","features":[],"optional":false,"default_features":true,"target":null,"kind":"normal"},{"name":"futures-util","req":"^0.3.31","features":[],"optional":false,"default_features":true,"target":null,"kind":"normal"},{"name":"reqwest","req":"^0.12","features":["json","stream"],"optional":false,"default_features":true,"target":null,"kind":"normal"},{"name":"reqwest-eventsource","req":"^0.6.0","features":[],"optional":false,"default_features":true,"target":null,"kind":"normal"},{"name":"serde","req":"^1.0.228","features":["derive"],"optional":false,"default_features":true,"target":null,"kind":"normal"},{"name":"serde_json","req":"^1.0.149","features":[],"optional":false,"default_features":true,"target":null,"kind":"normal"},{"name":"tokio","req":"^1.52.3","features":["macros","rt-multi-thread"],"optional":false,"default_features":true,"target":null,"kind":"normal"}],"cksum":"bf9e654e5c335bcfc27f10fc56a8e4a7bc8da740218eca8c3df338c3551552db","features":{},"yanked":false,"pubtime":"2026-05-25T08:00:57Z"}
    ```

    可以编写程序高效解析，提取每个 crate 的依赖关系。

#### 阶段二：构建依赖图并计算拓扑属性

获取数据后，核心任务是将数据转换为图结构并计算相关属性。

* **图构建**：通常将每个 crate 视为一个节点。若 crate A 依赖 crate B，则从 A 到 B 建立一条有向边。
* **工具与算法**：推荐使用 Python 的 `networkx` 库，并搭配 `powerlaw`等库进行拟合分析。
  * **度分布**：使用 `networkx.degree()` 获取全网的度序列。
  * **幂律检验**：通过 `powerlaw.Fit` 拟合度分布，检验其是否遵循幂律分布，以验证**无标度**假设。
  * **平均最短路径长度**：对于大图可抽样计算，实测值通常接近对数增长。
  * **聚类系数**：计算所有节点的平均聚类系数，实测值通常显著高于同等规模的随机网络。
  * **小世界检验**：构造一个具有相同节点数和边数的 **E-R随机图** 或 **WS模型图**。比较这两个随机图与真实依赖网络的平均路径长度 `L` 和平均聚类系数 `C`。
    * 若 \( \gamma \approx \frac{C_{\text{real}}}{C_{\text{random}}} \gg 1 \) 且 \( \lambda \approx \frac{L_{\text{real}}}{L_{\text{random}}} \approx 1 \)，则网络具有**小世界**特性。

#### 阶段三：高级分析与可视化

* **可视化**：利用 `matplotlib` 绘制度分布的**双对数坐标图**，以直观展示其幂律尾部。对于核心子图，可用 `networkx.draw` 或 `graphviz` 绘制**力导向布局图**，观察枢纽节点和社区。
* **中心度分析**：筛选出**PageRank**或**接近中心度**最高的 crate，这些即生态系统中的关键核心库。
* **社区发现**：应用 **Louvain** 或 **Girvan-Newman** 等社团检测算法，将网络分解为高内聚的功能模块。

#### 阶段四：课程设计的预期成果

通过此课题，可产出以下具体成果，构成课程的完整报告：

1. **数据集概览**：描述构建的网络规模（节点数、边数、直径等）。
2. **核心发现**：通过幂律拟合分析图、与随机模型的对比数据等证据，系统证明 `crates.io` 依赖网络具备**无标度**和**小世界**特征。
3. **关键节点识别**：列出最核心的若干 crate，分析其为何成为关键的枢纽。
4. **结构演化**（可选）：通过分析不同时间节点的快照数据，探讨网络特征随生态系统增长的演化趋势。

### 难度与风险提示

1. **计算资源**：完整的 `crates.io` 依赖图非常庞大，在PC上进行全图计算（如计算每对节点距离）几乎不可行。请设计合理的策略，或仅分析核心子图。
2. **数据解读**：幂律拟合需要统计知识，避免过度解读。

---

### 已实现的本地工作流

当前实现采用 **Rust 解析 + Python 分析** 的双流程：

1. **Rust** 解析 `crates.io-index`，仅保留每个 crate 的最新且未 yanked 版本；默认只统计 `normal` 依赖（可选开启 optional/dev/build）。
2. 基于 **in-degree** 选择核心子图（Top-N），输出 `core_nodes.csv` 与 `core_edges.csv`。
3. **Python** 对核心子图进行幂律拟合、小世界对比、中心度与社区分析，并输出图表与指标文件。

### 运行步骤

#### 1) Rust 解析索引

```bash
cargo run --release -- --index F:\crates.io-index --output outputs --top-n 20000
```

常用参数：

- `--include-optional`：包含 optional 依赖
- `--include-dev`：包含 dev 依赖
- `--include-build`：包含 build 依赖
- `--top-n <N>`：核心子图规模（默认 20000）
- `--output <DIR>`：输出目录（默认 outputs）

输出文件：

- `outputs/core_nodes.csv`
- `outputs/core_edges.csv`
- `outputs/summary.json`

#### 2) Python 分析

```bash
uv run python main.py --edges outputs/core_edges.csv --nodes outputs/core_nodes.csv --out-dir outputs/analysis --path-samples 50
```

输出文件：

- `outputs/analysis/metrics.json`
- `outputs/analysis/powerlaw.json`
- `outputs/analysis/small_world.json`
- `outputs/analysis/degree_in.png`
- `outputs/analysis/degree_out.png`
- `outputs/analysis/degree_total.png`
- `outputs/analysis/centrality.csv`
- `outputs/analysis/community_summary.json`

### 参数建议

- **Top-N**：建议 10k~30k 之间，保证可在 1 小时内完成。
- **路径长度抽样**：`--path-samples` 50~200 之间可权衡速度与稳定性。

### 报告大纲（可直接用作写作提纲）

1. 数据来源与预处理（索引来源、版本筛选规则、依赖类型过滤）
2. 核心子图选择策略（按 in-degree 排序，Top-N 规则）
3. 无标度检验（度分布图 + 幂律拟合参数）
4. 小世界检验（与随机图对比的 $\gamma$、$\lambda$）
5. 关键节点与社区结构（PageRank/社区划分结果）
6. 局限性与改进方向（抽样误差、参数敏感性）
