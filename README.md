<a id="readme-top"></a>
[![Build](https://github.com/jpochyla/psst/actions/workflows/build.yml/badge.svg)](https://github.com/jpochyla/psst/actions)


[![Contributors][contributors-shield]][contributors-url]
[![Forks][forks-shield]][forks-url]
[![Stargazers][stars-shield]][stars-url]
[![Issues][issues-shield]][issues-url]
[![MIT License][license-shield]][license-url]


<!-- PROJECT LOGO -->
<br />
<div align="center">
  <a href="https://github.com/jpochyla/psst">
    <img src="psst-gui/assets/logo_128.png" alt="Logo" width="80" height="80">
  </a>

<h3 align="center">Psst</h3>

  <p align="center">
    A fully cross-platform, fast Spotify client with a native GUI written in Rust, without Electron.
    <br />
    <b> A Spotify Premium account is required. </b>
    <br />
    <br />
    <a href="#builds">Download</a>
    ·
    <a href="https://github.com/github_username/repo_name/issues/new?labels=bug&template=bug-report---.md">Report Bug</a>
    ·
    <a href="https://github.com/github_username/repo_name/issues/new?labels=enhancement&template=feature-request---.md">Request Feature</a>
  </p>
</div>


<!-- TABLE OF CONTENTS -->
<details>
  <summary>Table of Contents</summary>
  <ol>
    <li>
      <a href="#about-the-project">About The Project</a>
      <ul>
        <li><a href="#builds">Download</a></li>
      </ul>
    </li>
    <li>
      <a href="#getting-started">Getting Started</a>
      <ul>
        <li><a href="#prerequisites">Prerequisites</a></li>
        <li><a href="#installation">Installation</a></li>
      </ul>
    </li>
    <li><a href="#usage">Usage</a></li>
    <li><a href="#roadmap">Roadmap</a></li>
    <li><a href="#contributing">Contributing</a></li>
    <li><a href="#license">License</a></li>
    <li><a href="#contact">Contact</a></li>
    <li><a href="#acknowledgments">Acknowledgments</a></li>
  </ol>
</details>



<!-- ABOUT THE PROJECT -->
## About The Project

[![Product Name Screen Shot][product-screenshot]](https://example.com)

Psst is a fast and mult-platform Spotify client, it is still under active development and big new features are on thir way, however, this means that stability and general user expereince may be limited. It is written in rust and uses the UI framework Druid. 
Contributions are welcome!

<p align="right">(<a href="#readme-top">back to top</a>)</p>



### Builds

GitHub Actions automatically creates builds when new commits are pushed to the `main` branch.
You can download the prebuilt binaries for x86_64 Windows, Linux (Ubuntu), and macOS.

| Platform                                                                                                            |
| ------------------------------------------------------------------------------------------------------------------- |
| [Linux (x86_64)](https://nightly.link/jpochyla/psst/workflows/build/main/psst-gui-x86_64-unknown-linux-gnu.zip)   |
| [Linux (aarch64)](https://nightly.link/jpochyla/psst/workflows/build/main/psst-gui-aarch64-unknown-linux-gnu.zip) |
| [Debian Package (amd64)](https://nightly.link/jpochyla/psst/workflows/build/main/psst-deb-amd64.zip)              |
| [Debian Package (arm64)](https://nightly.link/jpochyla/psst/workflows/build/main/psst-deb-arm64.zip)              |
| [MacOS](https://nightly.link/jpochyla/psst/workflows/build/main/Psst.dmg.zip)                                     |
| [Windows](https://nightly.link/jpochyla/psst/workflows/build/main/Psst.exe.zip)                                   |

Unofficial builds of Psst are also available through the [AUR](https://aur.archlinux.org/packages/psst-git) and [Homebrew](https://formulae.brew.sh/cask/psst).


<p align="right">(<a href="#readme-top">back to top</a>)</p>

### Prerequisites

On all platforms, the **latest [Rust](https://rustup.rs/) stable** (at least 1.65.0) is required.
For platform-specific requirements, see the dropdowns below.

<details>
<summary>Linux</summary>

Our user-interface library, Druid, has two possible backends on Linux: GTK and pure X11, with a Wayland backend in the works.
The default Linux backend is GTK.
Before building on Linux, make sure the required dependencies are installed.

#### Debian/Ubuntu:

```shell
sudo apt-get install libssl-dev libgtk-3-dev libcairo2-dev libasound2-dev
```

#### RHEL/Fedora:

```shell
sudo dnf install openssl-devel gtk3-devel cairo-devel alsa-lib-devel
```

</details>

<details>
<summary>OpenBSD (WIP)</summary>

OpenBSD support is still a WIP, and things will likely not function as intended.
Similar to Linux, Druid defaults to GTK while also providing a pure X11 backend.
Furthermore, bindgen must be able to find LLVM through the expected environment variable.
Only OpenBSD/amd64 has been tested so far.

```shell
doas pkg_add gtk+3 cairo llvm
export LIBCLANG_PATH=/usr/local/lib
```

In case rustc(1) fails building bigger crates

```shell
memory allocation of xxxx bytes failed
error: could not compile `gtk`
Caused by:
  process didn't exit successfully: `rustc --crate-name gtk [...]` (signal: 6, SIGABRT: process abort signal)
warning: build failed, waiting for other jobs to finish...
```

try increasing your user's maximum heap size:

```shell
ulimit -d $(( 2 * `ulimit -d` ))
```

</details>

---

### Installation

1. Clone the repo
```shell
git clone https://github.com/jpochyla/psst.git
```

3. Build from Source
```shell
cargo build
# Append `--release` for a release build.
```

4. Run from Source:
```shell
cargo run --bin psst-gui
# Append `--release` for a release build.
```

5. Build Installation Bundle (i.e., macOS .app):
```shell
cargo install cargo-bundle
cargo bundle --release
```

6. Change git remote url to avoid accidental pushes to base project
 ```sh
 git remote set-url origin github_username/repo_name
 git remote -v # confirm the changes
 ```

<p align="right">(<a href="#readme-top">back to top</a>)</p>


<!-- USAGE EXAMPLES -->
## Usage

Use this space to show useful examples of how a project can be used. Additional screenshots, code examples and demos work well in this space. You may also link to more resources.


<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- ROADMAP -->
## Roadmap

- [ ] Feature 1
- [ ] Feature 2
- [ ] Feature 3
    - [ ] Nested Feature

See the [open issues](https://github.com/jpochyla/psst/issues) for a full list of proposed features (and known issues).

<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- CONTRIBUTING -->
## Contributing

Contributions are what make the open source community such an amazing place to learn, inspire, and create. Any contributions you make are **greatly appreciated**.

If you have a suggestion that would make this better, please fork the repo and create a pull request. You can also simply open an issue with the tag "enhancement".
Don't forget to give the project a star! Thanks again!

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

<p align="right">(<a href="#readme-top">back to top</a>)</p>

### Top contributors:

<a href="https://github.com/github_username/jpochyla/psst/contributors">
  <img src="https://contrib.rocks/image?repo=jpochyla/psst" alt="contrib.rocks image" />
</a>



<!-- LICENSE -->
## License

Distributed under the MIT License. See `LICENSE.txt` for more information.

<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- CONTACT -->
## Contact

Your Name - [@twitter_handle](https://twitter.com/twitter_handle) - email@email_client.com

Project Link: [https://github.com/github_username/repo_name](https://github.com/jpochyla/psst)

<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- ACKNOWLEDGMENTS -->
## Acknowledgments

* []()
* []()
* []()

<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- MARKDOWN LINKS & IMAGES -->
<!-- https://www.markdownguide.org/basic-syntax/#reference-style-links -->
[contributors-shield]: https://img.shields.io/github/contributors/jpochyla/psst.svg?style=for-the-badge
[contributors-url]: https://github.com/jpochyla/psst/graphs/contributors
[forks-shield]: https://img.shields.io/github/forks/jpochyla/psst.svg?style=for-the-badge
[forks-url]: https://github.com/jpochyla/psst/network/members
[stars-shield]: https://img.shields.io/github/stars/jpochyla/psst.svg?style=for-the-badge
[stars-url]: https://github.com/jpochyla/psst/stargazers
[issues-shield]: https://img.shields.io/github/issues/jpochyla/psst.svg?style=for-the-badge
[issues-url]: https://github.com/jpochyla/psst/issues
[license-shield]: https://img.shields.io/github/license/jpochyla/psst.svg?style=for-the-badge
[license-url]: https://github.com/jpochyla/psst/blob/master/LICENSE.txt

[product-screenshot]: psst-gui/assets/screenshot.png
[Next.js]: https://img.shields.io/badge/next.js-000000?style=for-the-badge&logo=nextdotjs&logoColor=white
[Next-url]: https://nextjs.org/
[React.js]: https://img.shields.io/badge/React-20232A?style=for-the-badge&logo=react&logoColor=61DAFB
[React-url]: https://reactjs.org/
[Vue.js]: https://img.shields.io/badge/Vue.js-35495E?style=for-the-badge&logo=vuedotjs&logoColor=4FC08D
[Vue-url]: https://vuejs.org/
[Angular.io]: https://img.shields.io/badge/Angular-DD0031?style=for-the-badge&logo=angular&logoColor=white
[Angular-url]: https://angular.io/
[Svelte.dev]: https://img.shields.io/badge/Svelte-4A4A55?style=for-the-badge&logo=svelte&logoColor=FF3E00
[Svelte-url]: https://svelte.dev/
[Laravel.com]: https://img.shields.io/badge/Laravel-FF2D20?style=for-the-badge&logo=laravel&logoColor=white
[Laravel-url]: https://laravel.com
[Bootstrap.com]: https://img.shields.io/badge/Bootstrap-563D7C?style=for-the-badge&logo=bootstrap&logoColor=white
[Bootstrap-url]: https://getbootstrap.com
[JQuery.com]: https://img.shields.io/badge/jQuery-0769AD?style=for-the-badge&logo=jquery&logoColor=white
[JQuery-url]: https://jquery.com 
