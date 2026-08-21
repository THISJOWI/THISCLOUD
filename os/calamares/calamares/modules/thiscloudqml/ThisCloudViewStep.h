/* THISCLOUD install-config view step. */
#ifndef THISCLOUDVIEWSTEP_H
#define THISCLOUDVIEWSTEP_H

#include "utils/PluginFactory.h"
#include "viewpages/QmlViewStep.h"

#include <QObject>
#include <QString>

class ThisCloudConfig : public QObject
{
    Q_OBJECT
    Q_PROPERTY( QString nodeRole READ nodeRole WRITE setNodeRole NOTIFY nodeRoleChanged )
    Q_PROPERTY( QString clusterName READ clusterName WRITE setClusterName NOTIFY clusterNameChanged )
    Q_PROPERTY( QString nodeIp READ nodeIp WRITE setNodeIp NOTIFY nodeIpChanged )
    Q_PROPERTY( QString interface READ interface WRITE setInterface NOTIFY interfaceChanged )

public:
    explicit ThisCloudConfig( QObject* parent = nullptr );

    QString nodeRole() const { return m_nodeRole; }
    void setNodeRole( const QString& v ) { if ( v != m_nodeRole ) { m_nodeRole = v; emit nodeRoleChanged(); } }

    QString clusterName() const { return m_clusterName; }
    void setClusterName( const QString& v ) { if ( v != m_clusterName ) { m_clusterName = v; emit clusterNameChanged(); } }

    QString nodeIp() const { return m_nodeIp; }
    void setNodeIp( const QString& v ) { if ( v != m_nodeIp ) { m_nodeIp = v; emit nodeIpChanged(); } }

    QString interface() const { return m_interface; }
    void setInterface( const QString& v ) { if ( v != m_interface ) { m_interface = v; emit interfaceChanged(); } }

signals:
    void nodeRoleChanged();
    void clusterNameChanged();
    void nodeIpChanged();
    void interfaceChanged();

private:
    QString m_nodeRole = QStringLiteral( "worker" );
    QString m_clusterName = QStringLiteral( "thiscloud" );
    QString m_nodeIp = QStringLiteral( "127.0.0.1" );
    QString m_interface = QStringLiteral( "eth0" );
};

class ThisCloudViewStep : public Calamares::QmlViewStep
{
    Q_OBJECT

public:
    explicit ThisCloudViewStep( QObject* parent = nullptr );
    ~ThisCloudViewStep() override;

    QString prettyName() const override;
    void onLeave() override;
    bool isNextEnabled() const override;
    QObject* getConfig() override;

private:
    ThisCloudConfig* m_config = nullptr;
};

CALAMARES_PLUGIN_FACTORY_DECLARATION( ThisCloudViewStepFactory )

#endif