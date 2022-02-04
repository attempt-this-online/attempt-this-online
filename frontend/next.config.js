const nextBuildId = require('next-build-id');

module.exports = {
  generateBuildId: () => nextBuildId({ dir: __dirname, describe: true }),
  webpack: (config, {
    buildId, webpack,
  }) => {
    config.plugins.push(new webpack.DefinePlugin({ buildId: JSON.stringify(buildId) }));
    return config;
  },
  future: {
    webpack5: true,
  },
  eslint: {
    ignoreDuringBuilds: true,
  },
};
