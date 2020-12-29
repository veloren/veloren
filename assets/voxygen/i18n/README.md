# Translation document instructions

In order to keep localization documents readible please follow the following
rules:
- separate the string map sections using a commentary describing the purpose
  of the next section
- prepend multi-line strings with a commentary
- append one blank lines after a multi-line strings and two after sections


# Adding a new language in Veloren

To add a new language in Veloren, please follow these steps:
- Create a new folder into the `assets/voxygen/i18n` directory
- Copy the content of the `en` directory in your new folder
- Configure the language metadata in the `_manifest.ron` file
- From this point, you can start translating the files !
