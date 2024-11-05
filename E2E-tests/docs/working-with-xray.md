# How we work with XRay

## Regression tests

For each release:
1. Create a new XRay test plan for a release version, e.g., `v1.0.0`
2. Update the default XRay test plan at the .github/workflows to a new value.
3. Update the deployment version to a release version at the .github/workflows
4. Move the regression test plan to `Done` after a version is released.

Each Xray test execution will have corresponding environment and version. 

## Adding new tests

If you want to add a new test and connect it with XRay, we have two ways to do it. 

### Adding automated test first
1. Create an automated test
2. Run it in the scope of the regression test plan once
3. Find a new test in the latest test execution for an XRay test plan
4. Link automated test to XRay test by adding `@mark.test_key("ETCM-XXXX")` to test test method

### Adding XRay test first

1. Add new test with Generic type at Xray Test Repository
2. Add the corresponding automated test and link it to the XRay test by adding the `@mark.test_key("ETCM-XXXX")` to test method

## Custom runs

- To run automated tests and publish results to a custom test plan - use GitHub workflow for a chosen environment and specify the `XRay Test Plan` parameter
- To run automated tests and publish results to custom test execution - use GitHub workflow for a chosen environment and specify `XRay Test Execution` parameter