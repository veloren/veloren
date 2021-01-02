# Translation document instructions

In order to keep localization documents readible please follow the following
rules:
- Separate the string map sections using a commentary describing the purpose
  of the next section
- Prepend multi-line strings with a commentary
- Append one blank line after multi-line strings and two blank lines after sections


# Adding a new language in Veloren

To add a new language in Veloren, please follow these steps:
- Create a new folder into the `assets/voxygen/i18n` directory
- Copy the content of the `en` directory in your new folder
- Configure the language metadata in the `_manifest.ron` file
- From this point, you can start translating the files!
