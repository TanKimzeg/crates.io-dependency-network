# Crates.io 生态系统依赖网络的拓扑特性与鲁棒性分析——基于复杂网络视角的实证研究

[![CC BY-NC-SA 4.0][cc-by-nc-sa-shield]][cc-by-nc-sa]

[cc-by-nc-sa]: http://creativecommons.org/licenses/by-nc-sa/4.0/

[cc-by-nc-sa-shield]: https://img.shields.io/badge/License-CC%20BY--NC--SA%204.0-lightgrey.svg

---

## 摘要

本文运用复杂网络理论，对 Rust 语言官方包仓库 Crates.io 的依赖生态进行了系统性的实证分析。我构建了包含 265948 个节点和 1566132 条边的有向依赖网络，验证了其具有显著的**无标度**（入度幂律指数 α ≈ 1.87）和**小世界**（σ ≈ 3,754.84）特性。通过中心性分析，识别出 `serde` , `serde_json` , `thiserror` , `tokio` , `anyhow` 等对生态至关重要的“基础设施”节点。鲁棒性仿真实验揭示，该网络表现出“**面对随机故障鲁棒，面对蓄意攻击脆弱**”的经典无标度特征：模拟随机故障，曲线下降慢说明网络对随机失效更鲁棒。随机移除 60% 的节点仅导致巨片规模下降 48%，模拟攻击高连接节点，曲线几乎立刻接近 0。社区发现算法成功划分出148个功能模块，有效映射了 Rust 生态的技术分类。不同时期的演化分析显示，网络的幂律指数和平均路径长度等特征随时间趋于稳定。这些发现为理解及防范 Rust 生态的供应链风险提供了量化依据。

**关键词**：复杂网络；Crates.io；无标度网络；小世界特性；供应链风险；鲁棒性分析

---

## 1. 引言

Rust 是一门系统级编程语言，其首个稳定版 1.0 于 2015 年发布。该语言在不依赖垃圾回收机制的前提下，通过所有权（ownership）与借用检查器（borrow checker）等编译时约束，实现了与 C 语言相当的内存与运行时效率，同时保障了内存安全与线程安全。与多数传统系统语言不同，Rust 从设计初期便将包管理器 Cargo 作为语言生态的有机组成部分。Cargo 不仅负责依赖解析与构建流程，还统一了项目的目录结构与构建规范，从而为大规模静态分析和自动化工具链的构建提供了可复用的基础设施。作为 Cargo 的中央包仓库，[Crates.io](https://crates.io/) 构成了 Rust 生态系统的核心枢纽。截至 2026 年初，[Crates.io](https://crates.io/) 已托管超过 25 万个软件包（crates），日均新增上百个包，反映出 Rust 在工业与学术领域日益增长的采纳趋势。

随着生态系统的急剧膨胀，[Crates.io](https://crates.io/) 中的包之间通过 `Cargo.toml` 声明的依赖关系已经编织成一张极其庞大且紧密耦合的网络。每一个包的更新、废弃或安全事故，都有可能通过依赖链迅速传播，波及范围远超直觉预期。例如，当某个处于依赖底层的基础库被无意中移除（yank）或引入恶意代码时，数以万计的下游项目可能在一夜之间面临构建失败或安全威胁。因此，仅仅从单个包或局部依赖的角度审视软件生态，已不足以理解和防范这类系统性风险。要把握整个生态的宏观结构、演化趋势以及脆弱性，亟需一种能够从全局视角刻画依赖关系的分析框架。

近年来，将复杂网络理论应用于大规模软件生态系统分析已成为软件工程与网络科学交叉领域的热点方向。多项研究分别针对 npm、Maven、PyPI 等不同语言的包依赖网络，验证了其普遍存在的无标度（scale‑free）特性和小世界（small‑world）效应[^3]，并揭示出这些网络在面对随机故障时高度鲁棒、而在面对蓄意攻击关键枢纽节点时极为脆弱的双重特征[^4]。然而，针对 Rust 生态的此类宏观网络分析目前仍相对有限。[^5]Rust 语言特有的强类型约束、编译时安全检查以及 Cargo 对语义版本（semver）的严格遵循，是否使其依赖网络在拓扑结构上呈现出有别于其他生态的特异性？[^6]其供应链风险的分布又具有怎样的定量特征？这些问题尚缺乏系统的实证回答。

本文以复杂网络的视角，对 [Crates.io](https://crates.io/) 生态系统的依赖网络进行了一次较为全面的实证分析。我基于官方 `crates.io-index-archive` 仓库提供的全量版本发布信息，构建了以包名为节点、以实际正常依赖为边的有向依赖网络。在此网络基础上，本文依次完成了以下工作：（1）计算网络的基础拓扑属性，验证其是否具备无标度与小世界特征；（2）利用多种中心性指标识别对生态运转至关重要的关键枢纽节点；（3）通过鲁棒性仿真实验，量化网络在随机故障和蓄意攻击下的脆弱性差异；（4）运用社区发现算法挖掘功能模块，并结合技术分类进行解读。文章所获得的结果，旨在为理解 Rust 生态的供应链结构、评估潜在风险以及引导社区治理提供一套基于数据的量化依据（5）利用跨越多个时期的索引快照，从网络规模、幂律指数、平均路径长度等维度分析依赖网络的动态演化规律，并检验优先连接机制在 Rust 生态中的表现。文章所获得的结果，旨在为理解 Rust 生态的供应链结构、评估潜在风险、预测未来演化趋势以及引导社区治理提供一套基于数据的量化依据。

本文后续内容组织如下：第 2 节介绍数据来源、网络构建方式以及所用分析工具与指标；第 3 节详细报告各项实验结果并展开讨论，包括对多时期演化模式的专门分析；最后在第 4 节中总结全文并展望未来工作。

---

## 2. 数据与方法

### 2.1 数据来源

本文所用数据全部来自 Rust 官方包管理体系的公开仓库。具体而言，单个 crate 的版本发布记录（包括名称、版本、依赖声明等元信息）取自 `crates.io-index-archive` 这一官方 Git 仓库[^1]，该仓库以文件树形式存储了 [Crates.io](https://crates.io/) 自创立以来`crate.io-index`的部分快照。为分析crate其他关键字段信息，我还交叉参照了 `db-dump` 数据库转储[^2]，后者提供了每日更新的全量结构化快照，涵盖了 crate 的下载量、分类标签、首次发布时间等附属属性。

对于演化分析部分，我使用了 `crates.io-index-archive` 仓库所保留的不同历史快照。通过检索对应时期的 Git 树对象，重建了相隔约两年的五个时间切片（分别取自 2018-09-26、2020-11-20、2022-07-06、2024-03-11、及 2026-05-25 时刻附近的提交记录），从而获得随时间演化的多个网络版本。

### 2.2 网络构建规则

我将 [Crates.io](https://crates.io/) 生态系统抽象为一个有向图 $G=(V,E)$，其中：

- **节点**：为聚焦于包（crate）间的宏观关系并控制网络规模，本文以 **包名** 作为节点唯一标识。换言之，同一包的不同版本被聚合并为同一个节点，节点集 $V$ 的大小即为生态中已发布 crate 的数量。

- **有向边**：对于每个 crate 的每条发布记录，提取其 `deps` 字段中所有 `kind` 为 `"normal"` 且 `optional` 为 `false` 的依赖项。一条从 crate $A$ 指向 crate $B$ 的有向边 $A \to B$ 表示“$A$的正常构建必须依赖 $B$”，边的集合即反映了生态中真实、必需的编译时依赖关系。

我刻意排除了 `kind` 为 `"dev"` 的开发依赖和 `optional` 为 `true` 的可选依赖，因为前者仅在测试或示例代码中起作用，并不跟随主 crate 传播；后者默认不被启用，不具备强制的供应链传导效应。通过这一筛选，网络能够更加忠实地刻画“除非该边存在，否则下游无法编译”的硬依赖结构。

对于演化分析中的每个时间切片，我均以当时已发布的所有 crate 为节点集，并按照相同的规则抽取有效依赖边，从而构建出一系列随时间演化的有向网络 $G_1,G_{2},…,G_{T}​$ 。

### 2.3 分析工具与主要指标

网络构建与大部分图论运算基于 Python 的 `NetworkX`（v3.1）[^7] 完成；幂律分布检验借助 `powerlaw` 库 [^8]；网络可视化主要使用 `Gephi`（v0.11）[^9] 进行布局与着色，部分统计图则通过 `Matplotlib` 输出。而贯穿其中的数据清洗、提取等管道则通过Rust语言编程实现。本文分析所依赖的代码已经开源在[https://github.com/TanKimzeg/crates.io-dependency-network](https://github.com/TanKimzeg/crates.io-dependency-network)。

分析中涉及的主要网络度量指标及其定义如下：

| 指标                       | 含义                                                                                                    |
| ------------------------ | ----------------------------------------------------------------------------------------------------- |
| **度 & 度分布**              | 节点度数为与之相连的边数。有向网络中区分入度（被依赖数）与出度（依赖数）。度分布 `P(k)` 刻画度为 `k` 的节点所占比例                                      |
| **平均最短路径 L**             | 所有节点对之间最短路径长度的平均值，反映网络中信息或依赖传递的效率                                                                     |
| **平均聚类系数 C**             | 节点的邻居之间也互为邻居的平均概率，衡量网络的局部紧密程度                                                                         |
| **小世界系数 σ**              | `σ = (C/C_r) / (L/L_r)`，其中 `C_r` 与 `L_r` 为同等规模 ER 随机图的对应值。σ > 1 表明网络具有小世界效应                           |
| **中心性指标**                | - **入度中心性**：直观反映直接流行度<br>- **介数中心性**：节点位于最短路径上的频率，识别依赖链中的“桥梁”<br>- **PageRank**：考虑邻居权重的全局影响力，对有向网络更稳健 |
| **模块度 Q**                | 采用 Louvain 算法对网络的社区结构进行划分，并以模块度$Q$，值越接近 1 表示社区内部连接越紧密、社区之间越稀疏                                         |
| **巨片 (Giant Component)** | 网络中的最大连通分量。在鲁棒性实验中，其相对大小是衡量网络崩溃程度的核心指标                                                                |
| 网络演化相关指标                 | 追踪节点数 $N(t)$、平均度 $⟨k⟩_t$​、幂律指数 $α(t)$ 及平均最短路径 $L(t)$ 等指标随时间的变化趋势                                      |

演化分析部分，我还计算了各时期快照的**幂律指数 α** 以及**平均度**的变化趋势，并定量评估了**新增节点的依附偏好**（即新加入的 crate 连接到高度数节点的概率是否显著高于随机连接）。

### 2.4 实验设计概览

基于上述网络与指标，本文设计了以下五项实验以系统性回答研究问题：

1. **基础拓扑属性刻画**：计算并报告网络总体规模、密度、度分布、连通分量等基础参数。
2. **宏观特性检验**：通过幂律拟合（含 KS 检验）与小世界系数计算，验证网络的无标度及小世界特征。
3. **关键节点识别**：交叉比较三种中心性排序，定位生态中的核心基础设施级 crate。
4. **鲁棒性仿真**：为评估网络的容错与抗攻击能力，我实施了两类节点移除策略：模拟随机节点移除与基于中心性的蓄意攻击，记录最大连通分量相对大小的变化曲线。
5. **演化规律探索**：基于四个历史快照，我计算了各时期的拓扑统计量，绘制指标随时间的变化曲线，以揭示 [Crates.io](https://crates.io/) 生态的宏观演化态势。同时，为检验无标度网络理论中的**优先连接**机制是否在本生态中成立，我提取了每个时期内新增节点（即在前一个快照中不存在的 crate）在加入时刻所依附的旧节点的度值，并分析新节点选择连接目标的概率是否与目标节点的入度成正比。若正比关系近似成立，则表明生态成长遵循“富者愈富”的演化动力学。

以上实验的结果将在第 3 节中逐一展示并深入讨论。

---

## 3. 结果与分析

> 全文请见[Crates.io 生态系统依赖网络的拓扑特性与鲁棒性分析——基于复杂网络视角的实证研究](https://tankimzeg.top/blog/deep-learning/crates-io-dependency-network-analysis/)

---

## 参考文献

[^1]: Crates.io Index Archive. <https://github.com/rust-lang/crates.io-index-archive>

[^2]: Crates.io Database Dump. <https://static.crates.io/db-dump.tar.gz>

[^3]: Barabási A L, Albert R. Emergence of scaling in random networks[J]. science, 1999, 286(5439): 509-512.

[^4]: Ogenrwot D, Businge J, Arifuzzaman S. Structural and Connectivity Patterns in the Maven Central Software Dependency Network[C]//International Conference on Software Engineering and Data Engineering. Cham: Springer Nature Switzerland, 2025: 129-151.

[^5]: Hejderup J, Beller M, Triantafyllou K, et al. Präzi: from package-based to call-based dependency networks[J]. Empirical Software Engineering, 2022, 27(5): 102.

[^6]: Decan A, Mens T. What do package dependencies tell us about semantic versioning?[J]. IEEE Transactions on Software Engineering, 2019, 47(6): 1226-1240.

[^7]: Hagberg A, Swart P J, Schult D A. Exploring network structure, dynamics, and function using NetworkX[R]. Los Alamos National Laboratory (LANL), 2007.

[^8]: Alstott J, Bullmore E, Plenz D. powerlaw: a Python package for analysis of heavy-tailed distributions[J]. PloS one, 2014, 9(1): e85777.

[^9]: Bastian M, Heymann S, Jacomy M. Gephi: an open source software for exploring and manipulating networks[C]//Proceedings of the international AAAI conference on web and social media. 2009, 3(1): 361-362.
